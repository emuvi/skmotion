use scrap::{Capturer, Display};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{io, thread};
use webm::mux;
use webm::mux::Track;

#[derive(Debug, serde::Deserialize)]
pub struct Likes {
    pub display: usize,
    pub resolution: Option<(u32, u32)>,
    pub sensitivity: f64,
    pub resilience: u32,
    pub duration: Option<u64>,
    pub frames_ps: u64,
    pub bitrate: u32,
    pub destiny: PathBuf,
}

pub fn record(likes: Likes) -> io::Result<()> {
    let duration = likes.duration.map(Duration::from_secs);

    // Get the display.

    let displays = Display::all()?;

    let i = if displays.is_empty() {
        eprintln!("No displays found.");
        return Ok(());
    } else if displays.len() == 1 {
        0
    } else {
        let names: Vec<_> = displays
            .iter()
            .enumerate()
            .map(
                |(i, display)| format!("Display {} [{}x{}]", i, display.width(), display.height(),),
            )
            .collect();

        quest::ask("Which display?\n");
        let i = quest::choose(Default::default(), &names)?;
        println!();

        i
    };

    let display = displays.into_iter().nth(i).unwrap();

    // Setup the recorder.

    let mut capturer = Capturer::new(display)?;
    let width = capturer.width() as u32;
    let height = capturer.height() as u32;

    // Setup the multiplexer.

    let out = match {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&likes.destiny)
    } {
        Ok(file) => file,
        Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists => {
            if loop {
                quest::ask("Overwrite the existing file? [y/N] ");
                if let Some(b) = quest::yesno(false)? {
                    break b;
                }
            } {
                File::create(&likes.destiny)?
            } else {
                return Ok(());
            }
        }
        Err(e) => return Err(e.into()),
    };

    let mut webm =
        mux::Segment::new(mux::Writer::new(out)).expect("Could not initialize the multiplexer.");

    let vpx_codec = vpx_encode::VideoCodecId::VP9;
    let mux_codec = mux::VideoCodecId::VP9;

    let mut vt = webm.add_video_track(width, height, None, mux_codec);

    // Setup the encoder.

    let mut vpx = vpx_encode::Encoder::new(vpx_encode::Config {
        width: width,
        height: height,
        timebase: [1, 1000],
        bitrate: likes.bitrate,
        codec: vpx_codec,
    })
    .unwrap();

    // Start recording.

    let start = Instant::now();
    let stop = Arc::new(AtomicBool::new(false));

    thread::spawn({
        let stop = stop.clone();
        move || {
            let _ = quest::ask("Recording! Press ⏎ to stop.");
            let _ = quest::text();
            stop.store(true, Ordering::Release);
        }
    });

    let spf = Duration::from_nanos(1_000_000_000 / likes.frames_ps);
    let mut yuv = Vec::new();

    while !stop.load(Ordering::Acquire) {
        let now = Instant::now();
        let time = now - start;

        if Some(true) == duration.map(|d| time > d) {
            break;
        }

        match capturer.frame() {
            Ok(frame) => {
                let ms = time.as_secs() * 1000 + time.subsec_millis() as u64;

                crate::convert::argb_to_i420(width as usize, height as usize, &frame, &mut yuv);

                for frame in vpx.encode(ms as i64, &yuv).unwrap() {
                    vt.add_frame(frame.data, frame.pts as u64 * 1_000_000, frame.key);
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Wait.
            }
            Err(e) => {
                println!("{}", e);
                break;
            }
        }

        let dt = now.elapsed();
        if dt < spf {
            thread::sleep(spf - dt);
        }
    }

    // End things.

    let mut frames = vpx.finish().unwrap();
    while let Some(frame) = frames.next().unwrap() {
        vt.add_frame(frame.data, frame.pts as u64 * 1_000_000, frame.key);
    }

    let _ = webm.finalize(None);

    Ok(())
}

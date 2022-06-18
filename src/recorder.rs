use scrap::{Capturer, Display};
use std::fs::OpenOptions;
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
    let display = displays.into_iter().nth(likes.display).unwrap();

    // Setup the recorder.

    let mut capturer = Capturer::new(display)?;
    let width = capturer.width() as u32;
    let height = capturer.height() as u32;

    // Setup the multiplexer.

    let out = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&likes.destiny)
        .unwrap();

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
    let pause = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));

    thread::spawn({
        let pause = pause.clone();
        let stop = stop.clone();
        move || loop {
            let command = quest::text().unwrap();
            let command = command.trim();
            if command == "pause" {
                pause.store(true, Ordering::Release);
            } else if command == "continue" {
                pause.store(false, Ordering::Release);
            } else if command == "stop" {
                stop.store(true, Ordering::Release);
                break;
            }
        }
    });

    let spf = Duration::from_nanos(1_000_000_000 / likes.frames_ps);
    let mut yuv = Vec::new();

    while !stop.load(Ordering::Acquire) {
        if pause.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(10));
            continue;
        }
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

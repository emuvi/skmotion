use scrap::{Capturer, Display};
use webm::mux;
use webm::mux::Track;

use std::fs::OpenOptions;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::helper::AtomicF64;

#[derive(Debug)]
struct Like {
    display: usize,
    duration: Option<u64>,
    sensitivity: f64,
    resilience: u32,
    frames_ps: u64,
    bitrate: u32,
    destiny: PathBuf,
}

pub fn start(
    display: usize,
    duration: Option<u64>,
    sensitivity: f64,
    resilience: u32,
    frames_ps: u64,
    bitrate: u32,
    destiny: &str,
) -> std::io::Result<()> {
    record(Like {
        display,
        duration,
        sensitivity,
        resilience,
        frames_ps,
        bitrate,
        destiny: destiny.into(),
    })
}

fn record(like: Like) -> std::io::Result<()> {
    let duration = like.duration.map(Duration::from_secs);

    // Get the display.
    let displays = Display::all()?;
    let display = displays.into_iter().nth(like.display).unwrap();

    // Setup the recorder.

    let mut capturer = Capturer::new(display)?;
    let width = capturer.width() as u32;
    let height = capturer.height() as u32;

    // Setup the multiplexer.

    let out = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&like.destiny)
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
        bitrate: like.bitrate,
        codec: vpx_codec,
    })
    .unwrap();

    // Start recording.
    let like = Arc::new(like);
    let last_diff = Arc::new(AtomicF64::new(0.0));
    let frames_saved = Arc::new(AtomicU64::new(0));
    let frames_skipped = Arc::new(AtomicU64::new(0));
    let pause = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));

    std::thread::spawn({
        let like = like.clone();
        let last_diff = last_diff.clone();
        let frames_saved = frames_saved.clone();
        let frames_skipped = frames_skipped.clone();
        let pause = pause.clone();
        let stop = stop.clone();
        let stdin = std::io::stdin();
        move || {
            for line in stdin.lock().lines() {
                let command = line.unwrap();
                let command = command.trim();
                if command == "like" {
                    println!("{:?}", like);
                } else if command == "diff" {
                    println!("{}", last_diff.load(Ordering::Acquire));
                } else if command == "saved" {
                    println!("{}", frames_saved.load(Ordering::Acquire));
                } else if command == "skipped" {
                    println!("{}", frames_skipped.load(Ordering::Acquire));
                } else if command == "pause" {
                    pause.store(true, Ordering::Release);
                    println!("Paused");
                } else if command == "continue" {
                    pause.store(false, Ordering::Release);
                    println!("Continued");
                } else if command == "stop" {
                    stop.store(true, Ordering::Release);
                    println!("Stopped");
                    break;
                }
            }
        }
    });

    let nanos_time_base = 1_000_000_000 / like.frames_ps;
    let base_multiplier = std::cmp::max(like.frames_ps / 10, 1);
    let capture_interval = Duration::from_nanos(nanos_time_base);
    let mut last_saved = Vec::new();
    let mut yuv = Vec::new();

    let mut resilient = like.resilience;

    while !stop.load(Ordering::Acquire) {
        if pause.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        let start_cycle = Instant::now();
        let frame_index = frames_saved.load(Ordering::Acquire);
        let frame_time = Duration::from_nanos(base_multiplier * nanos_time_base * frame_index);
        if Some(true) == duration.map(|d| frame_time > d) {
            break;
        }

        let mut was_block = false;
        match capturer.frame() {
            Ok(frame) => {
                let diff = crate::helper::compare(&frame, &last_saved);
                last_diff.store(diff, Ordering::Release);
                let mut to_save = diff > like.sensitivity;
                if to_save {
                    resilient = like.resilience;
                    last_saved.clear();
                    for part in frame.iter() {
                        last_saved.push(*part);
                    }
                } else {
                    if resilient > 0 {
                        resilient -= 1;
                        to_save = true;
                    }
                }
                if to_save {
                    let ms = frame_time.as_secs() * 1000 + frame_time.subsec_millis() as u64;
                    crate::helper::argb_to_i420(width as usize, height as usize, &frame, &mut yuv);
                    for frame in vpx.encode(ms as i64, &yuv).unwrap() {
                        vt.add_frame(frame.data, frame.pts as u64 * 1_000_000, frame.key);
                    }
                    frames_saved.fetch_add(1, Ordering::AcqRel);
                } else {
                    frames_skipped.fetch_add(1, Ordering::AcqRel);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                was_block = true;
            }
            Err(e) => {
                eprintln!("{}", e);
                break;
            }
        }

        if !was_block {
            let cycle_elapsed = start_cycle.elapsed();
            if cycle_elapsed < capture_interval {
                std::thread::sleep(capture_interval - cycle_elapsed);
            }
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

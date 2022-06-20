use scrap::{Capturer, Display};
use webm::mux;
use webm::mux::Track;

use std::fs::OpenOptions;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

  let displays = Display::all()?;
  let display = displays.into_iter().nth(like.display).unwrap();

  let mut capturer = Capturer::new(display)?;
  let width = capturer.width() as u32;
  let height = capturer.height() as u32;

  let nanos_time_base = 1_000_000_000 / like.frames_ps;
  let base_multiplier = std::cmp::max(like.frames_ps / 10, 1);
  let capture_interval = Duration::from_nanos(nanos_time_base);

  let like = Arc::new(like);
  let frames_saved = Arc::new(AtomicU64::new(0));
  let frames_skipped = Arc::new(AtomicU64::new(0));
  let pause = Arc::new(AtomicBool::new(false));
  let stop = Arc::new(AtomicBool::new(false));

  let to_save_pool: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));

  let in_save = std::thread::spawn({
    let nanos_time_base = nanos_time_base;
    let base_multiplier = base_multiplier;

    let like = like.clone();
    let frames_saved = frames_saved.clone();
    let stop = stop.clone();
    let to_save_pool = to_save_pool.clone();
    move || {
      let out = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&like.destiny)
        .unwrap();
      let mut webm = mux::Segment::new(mux::Writer::new(out))
        .expect("Could not initialize the multiplexer.");
      let vpx_codec = vpx_encode::VideoCodecId::VP9;
      let mux_codec = mux::VideoCodecId::VP9;
      let mut vt = webm.add_video_track(width, height, None, mux_codec);
      let mut vpx = vpx_encode::Encoder::new(vpx_encode::Config {
        width: width,
        height: height,
        timebase: [1, 1000],
        bitrate: like.bitrate,
        codec: vpx_codec,
      })
      .unwrap();
      let mut yuv = Vec::new();
      loop {
        let frame = {
          let mut to_save_pool = to_save_pool.lock().unwrap();
          if to_save_pool.is_empty() {
            if stop.load(Ordering::Acquire) {
              break;
            }
            drop(to_save_pool);
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
          } else {
            to_save_pool.pop().unwrap()
          }
        };
        let frame_index = frames_saved.load(Ordering::Acquire);
        let frame_time = Duration::from_nanos(base_multiplier * nanos_time_base * frame_index);
        let ms = frame_time.as_secs() * 1000 + frame_time.subsec_millis() as u64;
        crate::helper::argb_to_i420(width as usize, height as usize, &frame, &mut yuv);
        for frame in vpx.encode(ms as i64, &yuv).unwrap() {
          vt.add_frame(frame.data, frame.pts as u64 * 1_000_000, frame.key);
        }
        println!("Insert frame");
        frames_saved.fetch_add(1, Ordering::AcqRel);
      }
      let mut frames = vpx.finish().unwrap();
      while let Some(frame) = frames.next().unwrap() {
        vt.add_frame(frame.data, frame.pts as u64 * 1_000_000, frame.key);
        println!("from where it come?")
      }
      let _ = webm.finalize(None);
    }
  });

  let to_check_pool: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));

  let in_check = std::thread::spawn({
    let like = like.clone();
    let frames_skipped = frames_skipped.clone();
    let stop = stop.clone();
    let to_save_pool = to_save_pool.clone();
    let to_check_pool = to_check_pool.clone();
    move || {
      let mut last_saved = Vec::new();
      let mut resilient = like.resilience;
      loop {
        let frame = {
          let mut to_check_pool = to_check_pool.lock().unwrap();
          if to_check_pool.is_empty() {
            if stop.load(Ordering::Acquire) {
              break;
            }
            drop(to_check_pool);
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
          } else {
            to_check_pool.pop().unwrap()
          }
        };
        println!("Receive to check");

        let mut to_save = crate::helper::is_different(&frame, &last_saved, like.sensitivity);
        if to_save {
          resilient = like.resilience;
          last_saved.clear();
          last_saved.extend_from_slice(&frame);
        } else {
          if resilient > 0 {
            resilient -= 1;
            to_save = true;
          }
        }
        if to_save {
          let mut to_save_pool = to_save_pool.lock().unwrap();
          to_save_pool.push(frame);
          println!("Send to save");
        } else {
          frames_skipped.fetch_add(1, Ordering::AcqRel);
        }
      }
    }
  });

  std::thread::spawn({
    let like = like.clone();
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

  let start_capture = Instant::now();
  while !stop.load(Ordering::Acquire) {
    if pause.load(Ordering::Acquire) {
      std::thread::sleep(Duration::from_millis(1));
      continue;
    }
    let start_cycle = Instant::now();
    if Some(true) == duration.map(|d| start_capture.elapsed() > d) {
      break;
    }
    let mut was_block = false;
    match capturer.frame() {
      Ok(frame) => {
        let to_check = Vec::from(&frame[..]);
        let mut to_check_pool = to_check_pool.lock().unwrap();
        to_check_pool.push(to_check);
        println!("Send to check");
      }
      Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
        was_block = true;
      }
      Err(e) => {
        eprintln!("{}", e);
      }
    }
    if !was_block {
      let cycle_elapsed = start_cycle.elapsed();
      if cycle_elapsed < capture_interval {
        std::thread::sleep(capture_interval - cycle_elapsed);
      }
    }
  }

  in_check.join().unwrap();
  in_save.join().unwrap();
  Ok(())
}

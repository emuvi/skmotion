use scrap::Display;

mod clip;
mod convert;
mod recorder;

fn displays() -> std::io::Result<()> {
    let displays = Display::all()?;
    for (i, display) in displays.into_iter().enumerate() {
        println!("Display {} [{}x{}]", i, display.width(), display.height());
    }
    Ok(())
}

fn record(display: usize, duration: Option<u64>, destiny: &str) -> std::io::Result<()> {
    let likes = recorder::Likes {
        display,
        sensitivity: 0.001,
        resilience: 27,
        duration,
        frames_ps: 30,
        bitrate: 5000,
        destiny: destiny.into(),
    };
    recorder::record(likes)
}

fn main() -> std::io::Result<()> {
    let args = clip::parse();
    if args.is_present("displays") {
        return displays();
    }
    let mut display = 0 as usize;
    if let Some(screen) = args.value_of("screen") {
        display = screen.parse::<usize>().unwrap();
    }
    let mut duration: Option<u64> = None;
    if let Some(extent) = args.value_of("extent") {
        duration = Some(extent.parse::<u64>().unwrap());
    }
    if args.is_present("record") {
        let destiny = args
            .value_of("record")
            .expect("Could not parse the record PATH argument.");
        return record(display, duration, destiny);
    }
    Ok(())
}

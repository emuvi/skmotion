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

fn record(display: usize, destiny: &str) -> std::io::Result<()> {
    let likes = recorder::Likes {
        display,
        resolution: None,
        sensitivity: 0.001,
        resilience: 27,
        duration: None,
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
    if let Some(screen_arg) = args.value_of("screen") {
        display = screen_arg.parse::<usize>().unwrap();
    }
    if args.is_present("record") {
        let destiny = args
            .value_of("record")
            .expect("Could not parse the record PATH argument.");
        return record(display, destiny);
    }
    Ok(())
}

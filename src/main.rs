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

fn record() -> std::io::Result<()> {
    let args = recorder::Args {
        arg_path: "test.webm".into(),
        flag_time: None,
        flag_fps: 30,
        flag_bv: 5000,
    };
    recorder::record(args)
}

fn main() -> std::io::Result<()> {
    let args = clip::parse();
    if args.is_present("displays") {
        return displays();
    }
    Ok(())
}

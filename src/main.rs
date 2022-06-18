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

fn main() -> std::io::Result<()> {
    let args = clip::parse();
    if args.is_present("displays") {
        return displays();
    }
    let mut display: usize = 0;
    if let Some(screen_arg) = args.value_of("screen") {
        display = screen_arg.parse::<usize>().unwrap();
    }
    let mut duration: Option<u64> = None;
    if let Some(extent_arg) = args.value_of("extent") {
        duration = Some(extent_arg.parse::<u64>().unwrap());
    }
    let mut sensitivity: f64 = 0.001;
    if let Some(sensitivity_arg) = args.value_of("sensitivity") {
        sensitivity = sensitivity_arg.parse::<f64>().unwrap();
    }
    if args.is_present("record") {
        let destiny = args
            .value_of("record")
            .expect("Could not parse the record PATH argument.");
        return recorder::start(display, duration, sensitivity, destiny);
    }
    Ok(())
}

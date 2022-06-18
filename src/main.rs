use scrap::Display;

mod clip;
mod helper;
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
    let mut resilience: u32 = 30;
    if let Some(resilience_arg) = args.value_of("resilience") {
        resilience = resilience_arg.parse::<u32>().unwrap();
    }
    let mut frames_ps: u64 = 30;
    if let Some(frames_ps_arg) = args.value_of("frames_ps") {
        frames_ps = frames_ps_arg.parse::<u64>().unwrap();
    }
    let mut bitrate: u32 = 7200;
    if let Some(bitrate_arg) = args.value_of("bitrate") {
        bitrate = bitrate_arg.parse::<u32>().unwrap();
    }
    if args.is_present("record") {
        let destiny = args
            .value_of("record")
            .expect("Could not parse the record PATH argument.");
        return recorder::start(
            display,
            duration,
            sensitivity,
            resilience,
            frames_ps,
            bitrate,
            destiny,
        );
    }
    Ok(())
}

use clap::{Arg, ArgMatches, Command};

pub fn parse() -> ArgMatches {
    Command::new("SkMotion")
		.version(clap::crate_version!())
		.about("SkMotion (Screen Motion) is a command program for desktop that records the frames of a screen only when there is motion on it.")
		.author("Éverton M. Vieira <emuvi@outlook.com.br>")
		.arg(
			Arg::new("displays")
				.short('d')
				.long("displays")
				.takes_value(false)
				.required(false)
				.help("Prints a list of all connected displays."),
		)
		.get_matches()
}

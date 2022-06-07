mod convert;
mod recorder;
mod userface;

fn simple_record() -> std::io::Result<()> {
    let args = recorder::Args {
        arg_path: "test.webm".into(),
        flag_codec: recorder::Codec::Vp9,
        flag_time: None,
        flag_fps: 30,
        flag_bv: 5000,
        flag_ba: 5,
    };
    recorder::record(args)
}

fn main() {
    userface::show();
}

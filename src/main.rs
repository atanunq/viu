use clap::{
    crate_description, crate_name, crate_version, value_parser, Arg,
    ArgAction::{Append, Help, SetTrue},
    Command,
};

mod app;
mod config;

use config::Config;

fn main() {
    let matches = Command::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .arg_required_else_help(true)
        .arg(
            Arg::new("file")
                .help("The images to be displayed. Set to - for standard input.")
                .action(Append),
        )
        .arg(
            Arg::new("width")
                .short('w')
                .long("width")
                .value_parser(value_parser!(u32))
                .help("Resize the image to a provided width"),
        )
        .arg(
            Arg::new("height")
                .short('h')
                .long("height")
                .value_parser(value_parser!(u32))
                .help("Resize the image to a provided height"),
        )
        .arg(
            Arg::new("x")
                .short('x')
                .default_value("0")
                .value_parser(value_parser!(u16))
                .help("X offset"),
        )
        .arg(
            Arg::new("y")
                .short('y')
                .default_value("0")
                .value_parser(value_parser!(i16))
                .help("Y offset"),
        )
        .arg(
            Arg::new("absolute-offset")
                .short('a')
                .long("absolute-offset")
                .action(SetTrue)
                .help("Make the x and y offset be relative to the top left terminal corner. If not set, they are relative to the cursor's position."),
        )
        .arg(
            Arg::new("recursive")
                .short('r')
                .long("recursive")
                .action(SetTrue)
                .help("Recurse down directories if passed one"),
        )
        .arg(
            Arg::new("blocks")
                .short('b')
                .long("blocks")
                .action(SetTrue)
                .help("Force block output"),
        )
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .action(SetTrue)
                .help("Output the name of the file before displaying"),
        )
        .arg(
            Arg::new("transparent")
                .short('t')
                .long("transparent")
                .action(SetTrue)
                .help("Display transparent images with transparent background"),
        )
        .arg(
            Arg::new("frames-per-second")
                .short('f')
                .long("frame-rate")
                .value_parser(value_parser!(u8))
                .help("Play the gif at a given frame rate"),
        )
        .arg(
            Arg::new("once")
                .short('1')
                .long("once")
                .action(SetTrue)
                .help("Loop only once through the gif"),
        )
        .arg(
            Arg::new("static")
                .short('s')
                .long("static")
                .action(SetTrue)
                .help("Show only the first frame of the gif"),
        )
        .disable_help_flag(true)
        .arg(
            Arg::new("help")
                .short('H')
                .long("help")
                .action(Help)
                .help("Print help information"),
        )
        .get_matches();

    let conf = Config::new(&matches);

    if let Err(e) = app::run(conf) {
        eprintln!("{:?}", e);
        std::process::exit(1);
    }
}

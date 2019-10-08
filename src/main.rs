use clap::{crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg};

mod app;
mod printer;

fn main() {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::with_name("FILE")
                .help("The image to be displayed")
                .multiple(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Output what is going on"),
        )
        .arg(
            Arg::with_name("name")
                .short("n")
                .long("name")
                .help("Output the name of the file before displaying"),
        )
        .arg(
            Arg::with_name("mirror")
                .short("m")
                .long("mirror")
                .help("Display a mirror of the original image"),
        )
        .arg(
            Arg::with_name("transparent")
                .short("t")
                .long("transparent")
                .help("Display transparent image with transparent background"),
        )
        .arg(
            Arg::with_name("once")
                .short("1")
                .long("once")
                .help("Only loop once through the animation"),
        )
        .arg(
            Arg::with_name("width")
                .short("w")
                .long("width")
                .takes_value(true)
                .help("Resize the image to a provided width"),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .takes_value(true)
                .help("Resize the image to a provided height"),
        )
        .arg(
            Arg::with_name("recursive")
                .short("r")
                .long("recursive")
                .help("Recurse down directories if passed one"),
        )
        .get_matches();

    let conf = app::Config::new(&matches);
    app::run(conf);
}

extern crate image;
extern crate clap;

use clap::{Arg, App};
use image::GenericImageView;


use std::env;
use std::process::Command;

fn main() {
    let matches = App::new("Experiment")
                        .version("1.0")
                        .author("Atanas Yankov")
                        .about("We will see what it does later on...")
                        .arg(Arg::with_name("print")
                             .short("p")
                             .long("print")
                             .help("Set image printing on when running the app"))
                        .get_matches();

    let img = image::open("bfa.jpg").unwrap();
    if matches.is_present("print") {
        let comm = Command::new("tiv")
            .arg("bfa.jpg")
            .spawn();

    }
}

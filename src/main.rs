extern crate image;
extern crate clap;

use clap::{Arg, App};

use std::fs;
use std::process::Command;

fn main() {
    let matches = App::new("Experiment")
        .version("1.0")
        .author("Atanas Yankov")
        .about("We will see what it does later on...")
        .arg(Arg::with_name("mirror")
             .short("m")
             .long("mirror")
             .help("Mirror the image"))
        .arg(Arg::with_name("overwrite")
             .short("o")
             .long("overwrite")
             .help("Set whether the original file should be overwritten"))
        .arg(Arg::with_name("FILE")
             .help("Set the image to manipulate")
             .required(true)
             .index(1))
        .get_matches();

    //load image only when needed
    let filename = matches.value_of("FILE").unwrap();
    let mut img = image::open(filename).unwrap();

    if matches.is_present("mirror") {
        img = img.fliph();
    }

    let tmp_name = format!("{}{}", "tmp.",filename);
    img.save(&tmp_name).expect("Failed to save temporary image.. Printing will not work!");
    let mut comm = Command::new("tiv");
    comm.arg(&tmp_name);
    comm.status().expect("Failed to print image!");

    fs::remove_file(&tmp_name).expect("Failed to delete temporary file!");

    if matches.is_present("overwrite") {
        img.save(filename).expect("Failed saving image!");
    }

}

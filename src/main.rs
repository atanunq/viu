extern crate image;
extern crate clap;

use clap::{Arg, App};
use image::{GenericImageView};

use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

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

    let p = img.get_pixel(100,100);
    //test
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Rgb(p.data[0], p.data[1], p.data[2]))).set_bold(true);
    stdout.set_color(&c);
    writeln!(&mut stdout, "green text!");


    if matches.is_present("mirror") {
        img = img.fliph();
    }

    let tmp_name = format!("{}{}", "tmp.",filename);
    img.save(&tmp_name).expect("Failed to save temporary image.. Printing will not work!");
    /*let mut comm = Command::new("tiv");
      comm.arg(&tmp_name);
      comm.status().expect("Failed to print image!");
      */
    fs::remove_file(&tmp_name).expect("Failed to delete temporary file!");

    if matches.is_present("overwrite") {
        img.save(filename).expect("Failed saving image!");
    }

}

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

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let mut c = ColorSpec::new();
    let (width, height) = img.dimensions();
    println!("{}x{}", width, height);

    let mut counter = 0;
    let chars = ["\u{2580}","\u{2581}","\u{2582}","\u{2583}","\u{2584}","\u{2585}","\u{2586}","\u{2587}","\u{2588}","\u{2589}",
    "\u{258A}", "\u{258B}", "\u{258C}", "\u{258D}", "\u{258E}", "\u{258F}", "\u{2590}"];
    for block in chars.iter() {
        println!("Trying with block: {}", block);
        for p in img.pixels() {
            counter = counter + 1;
            c.set_fg(Some(Color::Rgb(p.2[0], p.2[1], p.2[2]))).set_bold(false);
            stdout.set_color(&c);
            write!(&mut stdout, "{}", block);
            //or 258B
            if counter == width {
                writeln!(&mut stdout, "");
                counter = 0;
            }
        }
    }

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

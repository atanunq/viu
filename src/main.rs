extern crate clap;
extern crate image;

use clap::{value_t, App, Arg};
use image::{FilterType, GenericImageView};

mod printer;
mod size;

fn main() {
    let matches = App::new("Experiment")
        .version("1.0")
        .author("Atanas Yankov")
        .about("We will see what it does later on...")
        .arg(
            Arg::with_name("mirror")
                .short("m")
                .long("mirror")
                .help("Mirror the image"),
        )
        .arg(
            Arg::with_name("width")
                .short("w")
                .long("width")
                .takes_value(true)
                .help("Set the preferred width when displaying the image in the terminal"),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .takes_value(true)
                .help("Set the preferred height when displaying the image in the terminal"),
        )
        .arg(
            Arg::with_name("overwrite")
                .short("o")
                .long("overwrite")
                .help("Set whether the original file should be overwritten"),
        )
        .arg(
            Arg::with_name("FILE")
                .help("Set the image to manipulate")
                .required(true)
                .index(1),
        )
        .get_matches();

    let filename = matches.value_of("FILE").unwrap();
    let mut img = image::open(filename).unwrap();

    if matches.is_present("mirror") {
        img = img.fliph();
    }

    let print_img;
    let mut modified_img;
    let (width, height) = img.dimensions();
    let (mut print_width, mut print_height) = img.dimensions();

    let specified_width = matches.is_present("width");
    let specified_height = matches.is_present("height");

    if specified_width {
        let new_width = value_t!(matches, "width", u32).unwrap_or_else(|e| e.exit());
        print_width = new_width;
    }
    if specified_height {
        let new_height = value_t!(matches, "height", u32).unwrap_or_else(|e| e.exit());
        print_height = 2 * new_height;
    }
    if specified_width && specified_height {
        modified_img = img.thumbnail_exact(print_width, print_height);
        print_img = &modified_img;
    } else if specified_width || specified_height {
        modified_img = img.thumbnail(print_width, print_height);
        print_img = &modified_img;
    } else {
        match size::get_size() {
            Ok((w, h)) => {
                //only change values if the image needs to be resized (is bigger than the
                //terminal's size
                if width > w {
                    print_width = w;
                }
                if height > h {
                    print_height = 2 * h;
                }
            }
            Err(e) => {
                eprintln!("{}", e);
                //could not get terminal width => we fall back to a predefined value
                //maybe use env variable?
                print_width = 50;
            }
        };
        modified_img = img.resize(print_width, print_height, FilterType::Triangle);
        print_img = &modified_img;
    }

    printer::print(print_img);

    let (print_width, print_height) = print_img.dimensions();
    let (width, height) = img.dimensions();
    println!(
        "From {}x{} the image is now {}x{}",
        width, height, print_width, print_height
    );

    if matches.is_present("overwrite") {
        img.save(filename).expect("Failed saving image!");
    }
}

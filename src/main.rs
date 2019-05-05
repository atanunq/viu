extern crate clap;
extern crate image;

use clap::{value_t, App, Arg, ArgMatches};
use image::GenericImageView;
use std::io;
use std::io::prelude::*;

mod printer;
mod size;

//default width to be used when no options are passed and terminal size could not be computed
const DEFAULT_PRINT_WIDTH: u32 = 100;

fn main() {
    let matches = App::new("viu")
        .version("0.1")
        .author("Atanas Yankov")
        .about("View images right from the terminal.")
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
        .get_matches();
    run(matches);
}

fn run(matches: ArgMatches) {
    let files: Vec<&str> = match matches.values_of("FILE") {
        None => Vec::new(),
        Some(values) => values.collect(),
    };

    if files.len() > 0 {
        for filename in files.iter() {
            let img = match image::open(filename) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("\"{}\": {}", filename, e);
                    std::process::exit(1);
                }
            };

            if matches.is_present("name") {
                println!("{}:", filename);
            }

            process_image(img, &matches);
        }
    } else {
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        let mut buf: Vec<u8> = Vec::new();
        handle.read_to_end(&mut buf).unwrap();

        let img = image::load_from_memory(&buf).unwrap();
        process_image(img, &matches);
    }
}

fn process_image(img: image::DynamicImage, matches: &ArgMatches) {
    let verbose = matches.is_present("verbose");

    let mut print_img;
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
        //since 2 pixels are printed per terminal cell, an image with twice the height can be fit
        print_height = 2 * new_height;
    }
    if specified_width && specified_height {
        if verbose {
            println!(
                    "Both width and height are specified, resizing to {}x{} without preserving aspect ratio...",
                    print_width,
                    print_height
                );
        }
        print_img = img.thumbnail_exact(print_width, print_height);
    } else if specified_width || specified_height {
        if verbose {
            println!(
                    "Either width or height is specified, resizing to {}x{} and preserving aspect ratio...",
                    print_width, print_height
                );
        }
        print_img = img.thumbnail(print_width, print_height);
    } else {
        if verbose {
            println!(
                    "Neither width, nor height is specified, therefore terminal size will be matched..."
                );
        }
        match size::get_size() {
            Ok((w, h)) => {
                //only change values if the image needs to be resized
                //i.e is bigger than the terminal's size
                if width > w {
                    print_width = w;
                }
                if height > h {
                    print_height = 2 * h;
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("{}", e);
                }
                //could not get terminal width => we fall back to a predefined value
                //maybe use env variable?
                print_width = DEFAULT_PRINT_WIDTH;
            }
        };
        if verbose {
            println!(
                "Usable space is {}x{}, resizing and preserving aspect ratio...",
                print_width, print_height
            );
        }
        print_img = img.thumbnail(print_width, print_height);
    }

    if matches.is_present("mirror") {
        print_img = print_img.fliph();
    }

    printer::print(&print_img);

    let (print_width, print_height) = print_img.dimensions();
    let (width, height) = img.dimensions();
    if verbose {
        println!(
            "From {}x{} the image is now {}x{}",
            width, height, print_width, print_height
        );
    }
}

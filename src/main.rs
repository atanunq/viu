extern crate clap;
extern crate image;

use clap::{value_t, App, Arg, ArgMatches};
use image::{gif::Decoder, AnimationDecoder};
use image::{DynamicImage, GenericImageView, ImageRgba8};
use std::error::Error;
use std::fs::File;
use std::{thread, time};

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
                .required(true)
                .multiple(true)
                .empty_values(false),
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
    //TODO: create a config struct
    run(matches);
}

fn run(matches: ArgMatches) {
    let files: Vec<_> = matches.values_of("FILE").unwrap().collect();

    for filename in files.iter() {
        if matches.is_present("name") {
            println!("{}:", filename);
        }
        //TODO: only do that if we are sure the file is a legit image to avoid loading large files
        let file_in = File::open(filename);

        let file_in = match file_in {
            Err(e) => {
                file_missing_error(filename, e.description());
                panic!();
            }
            Ok(f) => f,
        };

        match Decoder::new(file_in) {
            Ok(decoder) => match decoder.into_frames().collect_frames() {
                Ok(frames) => {
                    let ten_millis = time::Duration::from_millis(10);
                    let mut is_first_frame = true;
                    //TODO: listen for user input to stop
                    loop {
                        for frame in &frames {
                            let buffer = frame.buffer();
                            //keep replacing old pixels as the gif goes on so that scrollback
                            //buffer is not filled
                            if !is_first_frame {
                                //TODO: rows should be cleared first
                                print!("{}[{}A", 27 as char, buffer.height());
                            } else {
                                is_first_frame = false;
                            }
                            handle_image(&matches, ImageRgba8(buffer.to_owned()));

                            thread::sleep(ten_millis);
                        }
                    }
                }
                Err(_) => {
                    if matches.is_present("verbose") {
                        println!(
                                "The GIF's frames could not be read, displaying only as an image instead."
                            );
                    }
                    print_simple_image(&matches, filename);
                }
            },
            Err(_) => {
                print_simple_image(&matches, filename);
            }
        }
    }
}

fn print_simple_image(matches: &ArgMatches, filename: &str) {
    match image::open(filename) {
        Ok(i) => {
            handle_image(&matches, i);
        }
        Err(e) => {
            file_missing_error(filename, e.description());
        }
    };
}

fn file_missing_error(filename: &str, e: &str) {
    eprintln!("\"{}\": {}", filename, e);
    std::process::exit(1);
}

fn handle_image(matches: &ArgMatches, img: DynamicImage) {
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

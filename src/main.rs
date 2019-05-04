extern crate clap;
extern crate ctrlc;
extern crate image;

use clap::{value_t, App, Arg, ArgMatches};
use image::{gif::Decoder, AnimationDecoder};
use image::{DynamicImage, GenericImageView, ImageRgba8};
use std::error::Error;
use std::fs::File;
use std::sync::mpsc;
use std::{thread, time::Duration};

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
    let conf = Config::new(&matches);
    run(conf);
}

struct Config<'a> {
    verbose: bool,
    name: bool,
    files: Vec<&'a str>,
    mirror: bool,
    width: Option<u32>,
    is_width_present: bool,
    height: Option<u32>,
    is_height_present: bool,
}

impl<'a> Config<'a> {
    fn new(matches: &'a ArgMatches) -> Config<'a> {
        let is_width_present = matches.is_present("width");
        let is_height_present = matches.is_present("height");
        let width = if is_width_present {
            Some(value_t!(matches, "width", u32).unwrap_or_else(|e| e.exit()))
        } else {
            None
        };
        let height = if is_height_present {
            Some(value_t!(matches, "height", u32).unwrap_or_else(|e| e.exit()))
        } else {
            None
        };

        Config {
            verbose: matches.is_present("verbose"),
            name: matches.is_present("name"),
            files: matches.values_of("FILE").unwrap().collect(),
            mirror: matches.is_present("mirror"),
            width,
            is_width_present,
            height,
            is_height_present,
        }
    }
}

fn run(conf: Config) {
    //create two channels so that ctrlc-handler and the main thread can pass messages in order to
    //communicate when printing must be stopped
    let (tx_ctrlc, rx_print) = mpsc::channel();
    let (tx_print, rx_ctrlc) = mpsc::channel();
    //handle Ctrl-C in order to clean up after ourselves
    ctrlc::set_handler(move || {
        //if ctrlc is received tell the infinite gif loop to stop drawing
        tx_ctrlc.send(true).unwrap();
        //a message will be received when that has happened so we can clear leftover symbols
        let _ = rx_ctrlc.recv().unwrap();
        print!("{}[0J", 27 as char);
        std::process::exit(0);
    })
    .expect("Could not setup Ctrl-C handler");

    let files_len = conf.files.len();

    for filename in conf.files.iter() {
        if conf.name {
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
                    let frames_len = frames.len();
                    let mut frame_counter;
                    let forty_millis = Duration::from_millis(40);
                    //TODO: listen for user input to stop, not only Ctrl-C
                    'infinite: loop {
                        frame_counter = 0;
                        for frame in &frames {
                            let buffer = frame.buffer();
                            let (_, height) = handle_image(&conf, ImageRgba8(buffer.to_owned()));
                            thread::sleep(forty_millis);

                            //if ctrlc is received then respond so the handler can clear the
                            //terminal from leftover colors
                            if rx_print.try_recv().is_ok() {
                                tx_print.send(true).unwrap();
                                break 'infinite;
                            };

                            frame_counter += 1;
                            //keep replacing old pixels as the gif goes on so that scrollback
                            //buffer is not filled
                            if frame_counter != frames_len {
                                print!("{}[{}A", 27 as char, height);
                            }
                        }
                        //only stop if there are other files to be previewed
                        //so that if only the gif is viewed, it will loop infinitely
                        if files_len != 1 {
                            break;
                        }
                    }
                }
                Err(_) => {
                    if conf.verbose {
                        println!(
                                "The GIF's frames could not be read, displaying only as an image instead."
                            );
                    }
                    print_simple_image(&conf, filename);
                }
            },
            Err(_) => {
                print_simple_image(&conf, filename);
            }
        }
    }
}

fn print_simple_image(conf: &Config, filename: &str) {
    match image::open(filename) {
        Ok(i) => {
            handle_image(conf, i);
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

fn handle_image(conf: &Config, img: DynamicImage) -> (u32, u32) {
    let mut print_img;
    let (width, height) = img.dimensions();
    let (mut print_width, mut print_height) = img.dimensions();

    if conf.is_width_present {
        print_width = conf.width.unwrap();
    }
    if conf.is_height_present {
        //since 2 pixels are printed per terminal cell, an image with twice the height can be fit
        print_height = 2 * conf.height.unwrap();
    }
    if conf.is_width_present && conf.is_height_present {
        if conf.verbose {
            println!(
                    "Both width and height are specified, resizing to {}x{} without preserving aspect ratio...",
                    print_width,
                    print_height
                );
        }
        print_img = img.thumbnail_exact(print_width, print_height);
    } else if conf.is_width_present || conf.is_height_present {
        if conf.verbose {
            println!(
                    "Either width or height is specified, resizing to {}x{} and preserving aspect ratio...",
                    print_width, print_height
                );
        }
        print_img = img.thumbnail(print_width, print_height);
    } else {
        if conf.verbose {
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
                if conf.verbose {
                    eprintln!("{}", e);
                }
                //could not get terminal width => we fall back to a predefined value
                //maybe use env variable?
                print_width = DEFAULT_PRINT_WIDTH;
            }
        };
        if conf.verbose {
            println!(
                "Usable space is {}x{}, resizing and preserving aspect ratio...",
                print_width, print_height
            );
        }
        print_img = img.thumbnail(print_width, print_height);
    }

    if conf.mirror {
        print_img = print_img.fliph();
    }

    printer::print(&print_img);

    let (print_width, print_height) = print_img.dimensions();
    let (width, height) = img.dimensions();
    if conf.verbose {
        println!(
            "From {}x{} the image is now {}x{}",
            width, height, print_width, print_height
        );
    }

    print_img.dimensions()
}

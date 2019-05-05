use clap::{value_t, ArgMatches};
use image::{gif::Decoder, AnimationDecoder};
use image::{DynamicImage, GenericImageView, ImageRgba8};
use std::fs::File;
use std::sync::mpsc;
use std::{thread, time::Duration};

use crate::printer;
use crate::size;

//default width to be used when no options are passed and terminal size could not be computed
const DEFAULT_PRINT_WIDTH: u32 = 100;

pub struct Config<'a> {
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
    pub fn new(matches: &'a ArgMatches) -> Config<'a> {
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

pub fn run(conf: Config) {
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

    let is_single_file = conf.files.len() == 1;

    //loop throught all files passed
    for filename in conf.files.iter() {
        if conf.name {
            println!("{}:", filename);
        }
        //TODO: only do that if we are sure the file is a legit image to avoid loading large files
        let file_in = File::open(filename);

        let file_in = match file_in {
            Err(e) => {
                error_and_quit(filename, e.to_string());
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
                            let (_, height) =
                                resize_and_print(&conf, ImageRgba8(buffer.to_owned()));
                            thread::sleep(forty_millis);

                            //if ctrlc is received then respond so the handler can clear the
                            //terminal from leftover colors
                            if rx_print.try_recv().is_ok() {
                                tx_print.send(true).unwrap();
                                break 'infinite;
                            };

                            frame_counter += 1;
                            //keep replacing old pixels as the gif goes on so that scrollback
                            //buffer is not filled (do not do that if its a sequence of files and
                            //one of them is a gif)
                            if frame_counter != frames_len || is_single_file {
                                print!("{}[{}A", 27 as char, height);
                            }
                        }
                        //only stop if there are other files to be previewed
                        //so that if only the gif is viewed, it will loop infinitely
                        if !is_single_file {
                            break;
                        }
                    }
                }
                Err(e) => {
                    if conf.verbose {
                        println!(
                                "The GIF's frames could not be read, displaying only as an image instead."
                            );
                    }
                    eprintln!("{}", e);
                    print_normal_image(&conf, filename);
                }
            },
            Err(_) => {
                //the provided image is not a gif so nothing special has to be done
                print_normal_image(&conf, filename);
            }
        }
    }
}

fn print_normal_image(conf: &Config, filename: &str) {
    match image::open(filename) {
        Ok(i) => {
            resize_and_print(conf, i);
        }
        Err(e) => {
            error_and_quit(filename, e.to_string());
        }
    };
}

fn error_and_quit(filename: &str, e: String) {
    eprintln!("\"{}\": {}", filename, e);
    std::process::exit(1);
}

fn resize_and_print(conf: &Config, img: DynamicImage) -> (u32, u32) {
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

use crate::printer;
use clap::{value_t, ArgMatches};
use crossterm::{terminal, Terminal};
use gif::SetParameter;
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageRgba8};
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::sync::mpsc;
use std::{thread, time::Duration};

pub struct Config<'a> {
    verbose: bool,
    name: bool,
    files: Vec<&'a str>,
    mirror: bool,
    transparent: bool,
    once: bool,
    width: Option<u32>,
    height: Option<u32>,
    terminal: Terminal,
}

impl<'a> Config<'a> {
    pub fn new(matches: &'a ArgMatches) -> Config<'a> {
        let width = if matches.is_present("width") {
            Some(value_t!(matches, "width", u32).unwrap_or_else(|e| e.exit()))
        } else {
            None
        };
        let height = if matches.is_present("height") {
            Some(value_t!(matches, "height", u32).unwrap_or_else(|e| e.exit()))
        } else {
            None
        };

        let files = match matches.values_of("FILE") {
            None => Vec::new(),
            Some(values) => values.collect(),
        };

        let terminal = terminal();

        Config {
            verbose: matches.is_present("verbose"),
            name: matches.is_present("name"),
            files,
            mirror: matches.is_present("mirror"),
            transparent: matches.is_present("transparent"),
            once: matches.is_present("once"),
            width,
            height,
            terminal,
        }
    }
}

pub fn run(conf: Config) {
    let no_files_passed = conf.files.is_empty();

    //create two channels so that ctrlc-handler and the main thread can pass messages in order to
    //communicate when printing must be stopped
    let (tx_ctrlc, rx_print) = mpsc::channel();
    let (tx_print, rx_ctrlc) = mpsc::channel();

    #[cfg(not(target_os = "wasi"))]
    {
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
    }

    //TODO: handle multiple files
    //TODO: maybe check an argument instead
    if no_files_passed {
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        let mut buf: Vec<u8> = Vec::new();
        let _ = handle.read_to_end(&mut buf).unwrap();

        if try_print_gif(&conf, BufReader::new(&*buf), &tx_print, &rx_print).is_err() {
            if let Ok(img) = image::load_from_memory(&buf) {
                resize_and_print(&conf, img);
            } else {
                let err = String::from("Data from stdin could not be decoded as an image.");
                error_and_quit("Stdin", err);
            };
        }
    }

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

        if try_print_gif(&conf, BufReader::new(file_in), &tx_print, &rx_print).is_err() {
            //the provided image is not a gif so nothing special has to be done
            print_normal_image(&conf, filename);
        }
    }
}

fn try_print_gif<R: Read>(
    conf: &Config,
    input_stream: R,
    tx: &mpsc::Sender<bool>,
    rx: &mpsc::Receiver<bool>,
) -> Result<(), gif::DecodingError> {
    //only stop if there are other files to be previewed
    //so that if only the gif is viewed, it will loop infinitely
    let should_loop = (conf.files.len() <= 1) && !conf.once;
    let mut decoder = gif::Decoder::new(input_stream);
    decoder.set(gif::ColorOutput::RGBA);
    match decoder.read_info() {
        //if it is a legit gif read the frames and start printing them
        Ok(mut decoder) => {
            let mut frames_vec = Vec::new();
            while let Some(frame) = decoder.read_next_frame().unwrap() {
                frames_vec.push(frame.to_owned());
            }
            let thirty_millis = Duration::from_millis(30);
            let frames_len = frames_vec.len();
            'infinite: loop {
                for (counter, frame) in frames_vec.iter().enumerate() {
                    //TODO: listen for user input to stop, not only Ctrl-C
                    let buffer = ImageBuffer::from_raw(
                        frame.width.into(),
                        frame.height.into(),
                        std::convert::From::from(frame.buffer.to_owned()),
                    )
                    .unwrap();
                    let (_, height) = resize_and_print(&conf, ImageRgba8(buffer));

                    #[cfg(not(target_os = "wasi"))]
                    {
                        thread::sleep(thirty_millis);

                        //if ctrlc is received then respond so the handler can clear the
                        //terminal from leftover colors
                        if rx.try_recv().is_ok() {
                            tx.send(true).unwrap();
                            break;
                        };
                    }

                    //keep replacing old pixels as the gif goes on so that scrollback
                    //buffer is not filled (do not do that if it is the last frame of the gif
                    //and a couple of files are being processed
                    if counter != frames_len - 1 || should_loop {
                        //since picture height is in pixel, we divide by 2 to get the height in
                        //terminal cells
                        print!("{}[{}A", 27 as char, height / 2 + height % 2);
                    }
                }
                if !should_loop {
                    break 'infinite;
                }
            }
            Ok(())
        }
        Err(e) => Err(e),
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

fn resize(conf: &Config, img: &DynamicImage) -> DynamicImage {
    let mut new_img;
    let (width, height) = img.dimensions();
    let (mut print_width, mut print_height) = img.dimensions();

    if let Some(w) = conf.width {
        print_width = w;
    }
    if let Some(h) = conf.height {
        //since 2 pixels are printed per terminal cell, an image with twice the height can be fit
        print_height = 2 * h;
    }
    match (conf.width, conf.height) {
        (None, None) => {
            if conf.verbose {
                println!(
                "Neither width, nor height is specified, therefore terminal size will be matched..."
            );
            }

            let size;
            match conf.terminal.size() {
                Ok(s) => {
                    size = s;
                }
                Err(e) => {
                    //If getting terminal size fails, fall back to some default size
                    size = (100, 40);
                    if conf.verbose {
                        eprintln!("{}", e);
                    }
                }
            }
            let (term_w, term_h) = size;
            let w = u32::from(term_w);
            //One less row because two reasons:
            // - the prompt after executing the command will take a line
            // - gifs flicker
            let h = u32::from(term_h - 1);
            if width > w {
                print_width = w;
            }
            if height > h {
                print_height = 2 * h;
            }
            if conf.verbose {
                println!(
                    "Usable space is {}x{}, resizing and preserving aspect ratio...",
                    print_width, print_height
                );
            }
            new_img = img.thumbnail(print_width, print_height);
        }
        (Some(_), None) | (None, Some(_)) => {
            if conf.verbose {
                println!(
                    "Either width or height is specified, resizing to {}x{} and preserving aspect ratio...",
                    print_width, print_height
                );
            }
            new_img = img.thumbnail(print_width, print_height);
        }
        (Some(_w), Some(_h)) => {
            if conf.verbose {
                println!(
                "Both width and height are specified, resizing to {}x{} without preserving aspect ratio...",
                print_width,
                print_height
            );
            }
            new_img = img.thumbnail_exact(print_width, print_height);
        }
    };

    if conf.mirror {
        new_img = new_img.fliph();
    };
    new_img
}

fn resize_and_print(conf: &Config, img: DynamicImage) -> (u32, u32) {
    let new_img = resize(conf, &img);

    printer::print(&new_img, conf.transparent);

    let (print_width, print_height) = new_img.dimensions();
    let (width, height) = img.dimensions();
    if conf.verbose {
        println!(
            "From {}x{} the image is now {}x{}",
            width, height, print_width, print_height
        );
    }

    new_img.dimensions()
}

#[cfg(test)]
mod test {
    use crate::app::{resize, Config};
    use crossterm::terminal;
    use image::GenericImageView;

    impl<'a> Config<'a> {
        fn test_config() -> Config<'a> {
            Config {
                verbose: false,
                name: false,
                files: vec!["img/bfa.jpg"],
                mirror: false,
                transparent: false,
                once: false,
                width: None,
                height: None,
                terminal: terminal(),
            }
        }
    }

    #[test]
    fn test_resize_with_none() {
        let conf = Config::test_config();
        match image::open("img/bfa.jpg") {
            Ok(i) => {
                //make sure the app doesn't panic without input
                let _img = resize(&conf, &i);
            }
            Err(_) => {
                panic!("Could not run resize test");
            }
        };
    }
    #[test]
    fn test_resize_only_width() {
        let mut conf = Config::test_config();
        conf.width = Some(200);
        match image::open("img/bfa.jpg") {
            Ok(i) => {
                let img = resize(&conf, &i);
                assert_eq!(img.dimensions(), (200, 112));
            }
            Err(_) => {
                panic!("Could not run resize test");
            }
        };
    }
    #[test]
    fn test_resize_only_height() {
        let mut conf = Config::test_config();
        conf.height = Some(20);
        match image::open("img/bfa.jpg") {
            Ok(i) => {
                let img = resize(&conf, &i);
                assert_eq!(img.dimensions(), (71, 40));
            }
            Err(_) => {
                panic!("Could not run resize test");
            }
        };
    }
    #[test]
    fn test_resize_given_both() {
        let mut conf = Config::test_config();
        conf.height = Some(20);
        conf.width = Some(200);
        match image::open("img/bfa.jpg") {
            Ok(i) => {
                let img = resize(&conf, &i);
                assert_eq!(img.dimensions(), (200, 40));
            }
            Err(_) => {
                panic!("Could not run resize test");
            }
        };
    }
}

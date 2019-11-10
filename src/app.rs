use crate::printer;
use clap::{value_t, ArgMatches};
use crossterm::terminal;
use gif::SetParameter;
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageRgba8};
use std::fs;
use std::io::{self, BufReader, Read};
use std::sync::mpsc;
use std::{thread, time::Duration};

pub struct Config<'a> {
    files: Vec<&'a str>,
    loop_gif: bool,
    verbose: bool,
    name: bool,
    mirror: bool,
    transparent: bool,
    recursive: bool,
    width: Option<u32>,
    height: Option<u32>,
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

        let once = matches.is_present("once");
        let loop_gif = files.len() <= 1 && !once;

        Config {
            files,
            loop_gif,
            verbose: matches.is_present("verbose"),
            name: matches.is_present("name"),
            mirror: matches.is_present("mirror"),
            transparent: matches.is_present("transparent"),
            recursive: matches.is_present("recursive"),
            width,
            height,
        }
    }
}

pub fn run(mut conf: Config) {
    //create two channels so that ctrlc-handler and the main thread can pass messages in order to
    // communicate when printing must be stopped without distorting the current frame
    let (tx_ctrlc, rx_print) = mpsc::channel();
    let (tx_print, rx_ctrlc) = mpsc::channel();

    #[cfg(not(target_os = "wasi"))]
    {
        //handle Ctrl-C in order to clean up after ourselves
        ctrlc::set_handler(move || {
            //if ctrlc is received tell the infinite gif loop to stop drawing
            // or stop the next file from being drawn
            tx_ctrlc.send(true).unwrap();
            //a message will be received when that has happened so we can clear leftover symbols
            let _ = rx_ctrlc.recv().unwrap();
            print!("{}[0J", 27 as char); // clear all symbols below the cursor
            std::process::exit(0);
        })
        .expect("Could not setup Ctrl-C handler");
    }

    //TODO: handle multiple files
    //TODO: maybe check an argument instead
    let no_files_passed = conf.files.is_empty();
    if no_files_passed {
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        let mut buf: Vec<u8> = Vec::new();
        let _ = handle.read_to_end(&mut buf).unwrap();

        if try_print_gif(&conf, "Stdin", BufReader::new(&*buf), &tx_print, &rx_print).is_err() {
            if let Ok(img) = image::load_from_memory(&buf) {
                resize_and_print(&conf, true, img);
            } else {
                let err = String::from("Data from stdin could not be decoded as an image.");
                //we want to exit the program => be verbose and have no tolerance
                error_and_quit("Stdin", err, true, false);
            };
        }
    } else {
        view_passed_files(&mut conf, &tx_print, &rx_print);
    }
}

fn view_passed_files(conf: &mut Config, tx: &mpsc::Sender<bool>, rx: &mpsc::Receiver<bool>) {
    //loop throught all files passed
    for filename in &conf.files {
        match fs::metadata(filename) {
            Ok(m) => {
                if m.is_dir() {
                    conf.loop_gif = false;
                    view_directory(conf, filename, tx, rx);
                } else {
                    view_file(conf, filename, false, tx, rx);
                }
            }
            Err(e) => eprintln!("{}", e),
        }
    }
}

fn view_directory(
    conf: &Config,
    dirname: &str,
    tx: &mpsc::Sender<bool>,
    rx: &mpsc::Receiver<bool>,
) {
    match fs::read_dir(dirname) {
        Ok(iter) => {
            for file in iter {
                //check if Ctrl-C has been received
                if rx.try_recv().is_ok() {
                    tx.send(true).unwrap();
                    break;
                };
                match file {
                    //check if the given file is a directory
                    Ok(f) => match f.metadata() {
                        Ok(m) => {
                            if m.is_dir() {
                                //if -r is passed, continue down
                                if conf.recursive {
                                    view_directory(conf, f.path().to_str().unwrap(), tx, rx);
                                }
                            }
                            //if it is a regular file, view it
                            else {
                                view_file(conf, f.path().to_str().unwrap(), true, tx, rx);
                            }
                        }
                        Err(e) => eprintln!("Could not fetch file metadata: {}", e),
                    },
                    Err(e) => eprintln!("Iterator failed to provide a DirEntry: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Could not get directory iterator: {}", e),
    }
}

//the tolerance argument specifies either if the program will
// - exit on error (when one of the passed files could not be viewed)
// - fail silently and continue (for a file in a directory)
fn view_file(
    conf: &Config,
    filename: &str,
    tolerant: bool,
    tx: &mpsc::Sender<bool>,
    rx: &mpsc::Receiver<bool>,
) {
    let file_in = match fs::File::open(filename) {
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
        Ok(f) => f,
    };

    //errors should be reported if -v is passed or if we do not tolerate them
    let should_report_err = conf.verbose || !tolerant;

    if try_print_gif(conf, filename, BufReader::new(file_in), tx, rx).is_err() {
        //the provided image is not a gif so try to view it
        match image::io::Reader::open(filename) {
            Ok(i) => match i.with_guessed_format() {
                Ok(img) => match img.decode() {
                    Ok(decoded) => {
                        if conf.name {
                            println!("{}:", filename);
                        }
                        resize_and_print(conf, true, decoded);
                    }
                    //Could not guess format
                    Err(e) => error_and_quit(filename, e.to_string(), should_report_err, tolerant),
                },

                Err(e) => error_and_quit(
                    filename,
                    format!("An IO error occured while docoding: {}", e),
                    should_report_err,
                    tolerant,
                ),
            },
            Err(e) => error_and_quit(
                filename,
                format!("Could not open file: {}", e),
                should_report_err,
                tolerant,
            ),
        };
    }
}

fn try_print_gif<R: Read>(
    conf: &Config,
    filename: &str,
    input_stream: R,
    tx: &mpsc::Sender<bool>,
    rx: &mpsc::Receiver<bool>,
) -> Result<(), gif::DecodingError> {
    let mut decoder = gif::Decoder::new(input_stream);
    decoder.set(gif::ColorOutput::RGBA);
    match decoder.read_info() {
        //if it is a legit gif read the frames and start printing them
        Ok(mut decoder) => {
            if conf.name {
                println!("{}:", filename);
            }
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
                    let (_, height) = resize_and_print(&conf, false, ImageRgba8(buffer));

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
                    // buffer is not filled (do not do that if it is the last frame of the gif
                    // and a couple of files are being processed
                    if counter != frames_len - 1 || conf.loop_gif {
                        //since picture height is in pixel, we divide by 2 to get the height in
                        // terminal cells
                        print!("{}[{}A", 27 as char, height / 2 + height % 2);
                    }
                }
                if !conf.loop_gif {
                    break 'infinite;
                }
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn error_and_quit(filename: &str, e: String, verbose: bool, tolerant: bool) {
    if verbose {
        eprintln!("\"{}\": {}", filename, e);
    }
    if !tolerant {
        std::process::exit(1);
    }
}

fn resize(conf: &Config, is_not_gif: bool, img: &DynamicImage) -> DynamicImage {
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
            if conf.verbose && is_not_gif {
                println!(
                    "Neither width, nor height is specified, therefore terminal size will be matched"
                );
            }

            let size;
            match terminal::size() {
                Ok(s) => {
                    size = s;
                }
                Err(e) => {
                    //If getting terminal size fails, fall back to some default size
                    size = (100, 40);
                    if conf.verbose && is_not_gif {
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
            if conf.verbose && is_not_gif {
                println!(
                    "Usable space is {}x{}, resizing and preserving aspect ratio",
                    print_width, print_height
                );
            }
            new_img = img.thumbnail(print_width, print_height);
        }
        (Some(_), None) | (None, Some(_)) => {
            if conf.verbose && is_not_gif {
                println!(
                    "Either width or height is specified, resizing to {}x{} and preserving aspect ratio",
                    print_width, print_height
                );
            }
            new_img = img.thumbnail(print_width, print_height);
        }
        (Some(_w), Some(_h)) => {
            if conf.verbose && is_not_gif {
                println!(
                    "Both width and height are specified, resizing to {}x{} without preserving aspect ratio",
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

fn resize_and_print(conf: &Config, is_not_gif: bool, img: DynamicImage) -> (u32, u32) {
    let new_img = resize(conf, is_not_gif, &img);

    printer::print(&new_img, conf.transparent);

    let (print_width, print_height) = new_img.dimensions();
    let (width, height) = img.dimensions();
    if conf.verbose && is_not_gif {
        println!(
            "From {}x{} the image is now {}x{}",
            width, height, print_width, print_height
        );
    }

    new_img.dimensions()
}

#[cfg(test)]
mod test {
    use crate::app::{resize, view_file, Config};
    use image::GenericImageView;
    use std::sync::mpsc;

    impl<'a> Config<'a> {
        fn test_config() -> Config<'a> {
            Config {
                files: vec![],
                loop_gif: true,
                verbose: false,
                name: false,
                mirror: false,
                transparent: false,
                recursive: false,
                width: None,
                height: None,
            }
        }
    }

    #[test]
    fn test_resize_with_none() {
        let conf = Config::test_config();
        match image::open("img/bfa.jpg") {
            Ok(i) => {
                //make sure the app doesn't panic without input
                let _img = resize(&conf, true, &i);
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
                let img = resize(&conf, true, &i);
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
                let img = resize(&conf, true, &i);
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
                let img = resize(&conf, true, &i);
                assert_eq!(img.dimensions(), (200, 40));
            }
            Err(_) => {
                panic!("Could not run resize test");
            }
        };
    }

    #[test]
    fn test_view_without_extension() {
        let conf = Config::test_config();
        let (tx, rx) = mpsc::channel();
        view_file(&conf, "img/bfa", false, &tx, &rx);
    }
}

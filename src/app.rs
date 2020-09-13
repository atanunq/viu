use crate::config::Config;
use crate::printer;
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{cursor, execute};
use image::{gif::GifDecoder, AnimationDecoder, DynamicImage, GenericImageView};
use std::fs;
use std::io::{stdin, stdout, BufReader, Read, Write};
use std::sync::mpsc;
use std::{thread, time::Duration};

const THIRTY_MILLIS: Duration = Duration::from_millis(30);

type TxRx<'a> = (&'a mpsc::Sender<bool>, &'a mpsc::Receiver<bool>);

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
            tx_ctrlc
                .send(true)
                .expect("Could not send signal to stop drawing.");
            //a message will be received when that has happened so we can clear leftover symbols
            let _ = rx_ctrlc
                .recv()
                .expect("Could not receive signal to clean up terminal.");

            execute!(stdout(), Clear(ClearType::FromCursorDown)).unwrap();
            std::process::exit(0);
        })
        .expect("Could not setup Ctrl-C handler");
    }

    //TODO: handle multiple files
    //read stdin if only 1 parameter is passed and it is "-"
    let should_read_stdin = conf.files.len() == 1 && conf.files[0] == "-";
    if should_read_stdin {
        let stdin = stdin();
        let mut handle = stdin.lock();

        let mut buf: Vec<u8> = Vec::new();
        let _ = handle
            .read_to_end(&mut buf)
            .expect("Could not read until EOF.");

        if try_print_gif(&conf, BufReader::new(&*buf), (&tx_print, &rx_print)).is_err() {
            if let Ok(img) = image::load_from_memory(&buf) {
                let new_img = resize(&conf, true, &img);

                printer::print(&new_img, conf.transparent, conf.truecolor);
            } else {
                let err = String::from("Data from stdin could not be decoded as an image.");
                //we want to exit the program => be verbose and have no tolerance
                error_and_quit("Stdin", err, true, false);
            };
        }
    } else {
        view_passed_files(&mut conf, (&tx_print, &rx_print));
    }
}

fn view_passed_files(conf: &mut Config, (tx, rx): TxRx) {
    //loop throught all files passed
    for filename in &conf.files {
        //check if Ctrl-C has been received. If yes, stop iterating
        if rx.try_recv().is_ok() {
            tx.send(true)
                .expect("Could not send signal to clean up terminal.");
            break;
        };
        match fs::metadata(filename) {
            Ok(m) => {
                //if its a directory, stop gif looping because there will probably be more files
                if m.is_dir() {
                    conf.loop_gif = false;
                    view_directory(conf, filename, (tx, rx));
                }
                //if a file has been passed individually and fails, do so intolerantly
                else {
                    view_file(conf, filename, false, (tx, rx));
                }
            }
            Err(e) => eprintln!("{}", e),
        }
    }
}

fn view_directory(conf: &Config, dirname: &str, (tx, rx): TxRx) {
    match fs::read_dir(dirname) {
        Ok(iter) => {
            for dir_entry_result in iter {
                //check if Ctrl-C has been received. If yes, stop iterating
                if rx.try_recv().is_ok() {
                    tx.send(true)
                        .expect("Could not send signal to clean up terminal.");
                    break;
                };
                match dir_entry_result {
                    //check if the given file is a directory
                    Ok(dir_entry) => match dir_entry.metadata() {
                        Ok(metadata) => {
                            if let Some(path_name) = dir_entry.path().to_str() {
                                if metadata.is_dir() {
                                    //if -r is passed, continue down
                                    if conf.recursive {
                                        view_directory(conf, path_name, (tx, rx));
                                    }
                                }
                                //if it is a regular file, viu it with tolerance = true
                                else {
                                    view_file(conf, path_name, true, (tx, rx));
                                }
                            } else {
                                eprintln!("Could not get path name, skipping...");
                                continue;
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
fn view_file(conf: &Config, filename: &str, tolerant: bool, (tx, rx): TxRx) {
    if conf.name {
        println!("{}:", filename);
    }
    let file_in = match fs::File::open(filename) {
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
        Ok(f) => f,
    };

    //errors should be reported if -v is passed or if we do not tolerate them
    let should_report = conf.verbose || !tolerant;

    if try_print_gif(conf, BufReader::new(file_in), (tx, rx)).is_err() {
        //the provided image is not a gif so try to view it
        match image::io::Reader::open(filename) {
            Ok(i) => match i.with_guessed_format() {
                Ok(img) => match img.decode() {
                    Ok(decoded) => {
                        let new_img = resize(conf, true, &decoded);

                        printer::print(&new_img, conf.transparent, conf.truecolor);
                    }
                    //Could not guess format
                    Err(e) => error_and_quit(filename, e.to_string(), should_report, tolerant),
                },

                Err(e) => error_and_quit(
                    filename,
                    format!("An IO error occured while decoding: {}", e),
                    should_report,
                    tolerant,
                ),
            },
            Err(e) => error_and_quit(
                filename,
                format!("Could not open file: {}", e),
                should_report,
                tolerant,
            ),
        };
    }
}

fn try_print_gif<R: Read>(
    conf: &Config,
    input_stream: R,
    (tx, rx): TxRx,
) -> Result<(), image::ImageError> {
    let decoder = GifDecoder::new(input_stream)?;
    //read all frames of the gif and resize them all at once before starting to print them
    let resized_frames: Vec<DynamicImage> = decoder
        .into_frames()
        .collect_frames()?
        .into_iter()
        .map(|f| resize(conf, false, &DynamicImage::ImageRgba8(f.into_buffer())))
        .collect();

    'infinite: loop {
        let mut iter = resized_frames.iter().peekable();
        while let Some(frame) = iter.next() {
            let height = frame.height();
            printer::print(&frame, conf.transparent, conf.truecolor);

            if conf.static_gif {
                break 'infinite;
            }

            #[cfg(not(target_os = "wasi"))]
            {
                thread::sleep(THIRTY_MILLIS);

                //if ctrlc is received then respond so the handler can clear the
                // terminal from leftover colors
                if rx.try_recv().is_ok() {
                    tx.send(true)
                        .expect("Could not send signal to clean up terminal");
                    break;
                };
            }

            //keep replacing old pixels as the gif goes on so that scrollback
            // buffer is not filled (do not do that if it is the last frame of the gif
            // or a couple of files are being processed)
            if iter.peek().is_some() || conf.loop_gif {
                //since picture height is in pixel, we divide by 2 to get the height in
                // terminal cells
                let up_lines = (height / 2 + height % 2) as u16;
                execute!(stdout(), cursor::MoveUp(up_lines)).unwrap();
            }
        }
        if !conf.loop_gif {
            break 'infinite;
        }
    }
    Ok(())
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
    let should_report = conf.verbose && is_not_gif;

    let mut new_img;
    let (width, height) = img.dimensions();
    let (mut print_width, mut print_height) = (width, height);

    if let Some(w) = conf.width {
        print_width = w;
    }
    if let Some(h) = conf.height {
        //since 2 pixels are printed per terminal cell, an image with twice the height can be fit
        print_height = 2 * h;
    }
    match (conf.width, conf.height) {
        (None, None) => {
            if should_report {
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
                    if should_report {
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
            if should_report {
                println!(
                    "Usable space is {}x{}, resizing and preserving aspect ratio",
                    print_width, print_height
                );
            }
            new_img = img.thumbnail(print_width, print_height);
        }
        (Some(_), None) | (None, Some(_)) => {
            if should_report {
                println!(
                    "Either width or height is specified, resizing to {}x{} and preserving aspect ratio",
                    print_width, print_height
                );
            }
            new_img = img.thumbnail(print_width, print_height);
        }
        (Some(_w), Some(_h)) => {
            if should_report {
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

    if should_report {
        println!(
            "From {}x{} the image is now {}x{}",
            width, height, print_width, print_height
        );
    }
    new_img
}

#[cfg(test)]
mod test {
    use super::*;

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
        view_file(&conf, "img/bfa", false, (&tx, &rx));
    }
}

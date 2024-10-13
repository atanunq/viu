use crate::config::Config;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, execute};
use image::{codecs::gif::GifDecoder, AnimationDecoder, DynamicImage};
use std::fs;
use std::io::{stdin, stdout, BufRead, BufReader, Cursor, Error, ErrorKind, Read, Seek};
use std::sync::mpsc;
use std::{thread, time::Duration};
use viuer::ViuResult;

type TxRx<'a> = (&'a mpsc::Sender<bool>, &'a mpsc::Receiver<bool>);

// TODO: Create a viu-specific result and error types, do not reuse viuer's
pub fn run(mut conf: Config) -> ViuResult {
    //create two channels so that ctrlc-handler and the main thread can pass messages in order to
    // communicate when printing must be stopped without distorting the current frame
    let (tx_ctrlc, rx_print) = mpsc::channel();
    let (tx_print, rx_ctrlc) = mpsc::channel();

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

        if let Err(e) = execute!(stdout(), Clear(ClearType::FromCursorDown)) {
            if e.kind() == ErrorKind::BrokenPipe {
                //Do nothing. Output is probably piped to `head` or a similar tool
            } else {
                panic!("{}", e);
            }
        }
        std::process::exit(0);
    })
    .map_err(|_| Error::new(ErrorKind::Other, "Could not setup Ctrl-C handler."))?;

    //TODO: handle multiple files
    //read stdin if only one parameter is passed and it is "-"
    if conf.files.len() == 1 && conf.files[0] == "-" {
        let stdin = stdin();
        let mut handle = stdin.lock();

        let mut buf: Vec<u8> = Vec::new();
        let _ = handle.read_to_end(&mut buf)?;
        let cursor = Cursor::new(&buf);

        //TODO: print_from_file if data is a gif and terminal is iTerm
        if try_print_gif(&conf, cursor, (&tx_print, &rx_print)).is_err() {
            //If stdin data is not a gif, treat it as a regular image

            let img = image::load_from_memory(&buf)?;
            viuer::print(&img, &conf.viuer_config)?;
        };

        Ok(())
    } else {
        view_passed_files(&mut conf, (&tx_print, &rx_print))
    }
}

fn view_passed_files(conf: &mut Config, (tx, rx): TxRx) -> ViuResult {
    //loop throught all files passed
    for filename in &conf.files {
        //check if Ctrl-C has been received. If yes, stop iterating
        if rx.try_recv().is_ok() {
            return tx.send(true).map_err(|_| {
                Error::new(ErrorKind::Other, "Could not send signal to clean up.").into()
            });
        };
        //if it's a directory, stop gif looping because there will probably be more files
        if fs::metadata(filename)?.is_dir() {
            conf.loop_gif = false;
            view_directory(conf, filename, (tx, rx))?;
        }
        //if a file has been passed individually and fails, propagate the error
        else {
            view_file(conf, filename, (tx, rx))?;
        }
    }
    Ok(())
}

fn view_directory(conf: &Config, dirname: &str, (tx, rx): TxRx) -> ViuResult {
    for dir_entry_result in fs::read_dir(dirname)? {
        //check if Ctrl-C has been received. If yes, stop iterating
        if rx.try_recv().is_ok() {
            return tx.send(true).map_err(|_| {
                Error::new(ErrorKind::Other, "Could not send signal to clean up.").into()
            });
        };
        let dir_entry = dir_entry_result?;

        //check if the given file is a directory
        if let Some(path_name) = dir_entry.path().to_str() {
            //if -r is passed, continue down
            if conf.recursive && dir_entry.metadata()?.is_dir() {
                view_directory(conf, path_name, (tx, rx))?;
            }
            //if it is a regular file, viu it, but do not exit on error
            else {
                let _ = view_file(conf, path_name, (tx, rx));
            }
        } else {
            eprintln!("Could not get path name, skipping...");
            continue;
        }
    }

    Ok(())
}

fn view_file(conf: &Config, filename: &str, (tx, rx): TxRx) -> ViuResult {
    if conf.name {
        println!("{}:", filename);
    }
    let mut file_in = fs::File::open(filename)?;

    // Read some of the first bytes to guess the image format
    let mut format_guess_buf: [u8; 20] = [0; 20];
    let _ = file_in.read(&mut format_guess_buf)?;
    // Reset the cursor
    file_in.seek(std::io::SeekFrom::Start(0))?;

    // If the file is a gif, let iTerm handle it natively
    if conf.viuer_config.use_iterm
        && viuer::is_iterm_supported()
        && (image::guess_format(&format_guess_buf[..])?) == image::ImageFormat::Gif
    {
        viuer::print_from_file(filename, &conf.viuer_config)?;
    } else {
        let result = try_print_gif(conf, BufReader::new(file_in), (tx, rx));
        //the provided image is not a gif so try to view it
        if result.is_err() {
            viuer::print_from_file(filename, &conf.viuer_config)?;
        }
    }

    Ok(())
}

fn try_print_gif<R>(conf: &Config, input_stream: R, (tx, rx): TxRx) -> ViuResult
where
    R: Read + BufRead + Seek,
{
    //read all frames of the gif and resize them all at once before starting to print them
    let resized_frames: Vec<(Duration, DynamicImage)> = GifDecoder::new(input_stream)?
        .into_frames()
        .collect_frames()?
        .into_iter()
        .map(|f| {
            let delay = Duration::from(f.delay());
            // Keep the image as it is for Kitty and iTerm, it will be printed in full resolution there
            if (conf.viuer_config.use_iterm && viuer::is_iterm_supported())
                || (conf.viuer_config.use_kitty
                    && viuer::get_kitty_support() != viuer::KittySupport::None)
            {
                (delay, DynamicImage::ImageRgba8(f.into_buffer()))
            } else {
                (
                    delay,
                    viuer::resize(
                        &DynamicImage::ImageRgba8(f.into_buffer()),
                        conf.viuer_config.width,
                        conf.viuer_config.height,
                    ),
                )
            }
        })
        .collect();

    'infinite: loop {
        let mut iter = resized_frames.iter().peekable();
        while let Some((delay, frame)) = iter.next() {
            let (_print_width, print_height) = viuer::print(frame, &conf.viuer_config)?;

            if conf.static_gif {
                break 'infinite;
            }

            thread::sleep(match conf.frame_duration {
                None => *delay,
                Some(duration) => duration,
            });

            //if ctrlc is received then respond so the handler can clear the
            // terminal from leftover colors
            if rx.try_recv().is_ok() {
                return tx.send(true).map_err(|_| {
                    Error::new(ErrorKind::Other, "Could not send signal to clean up.").into()
                });
            };

            //keep replacing old pixels as the gif goes on so that scrollback
            // buffer is not filled (do not do that if it is the last frame of the gif
            // or a couple of files are being processed)
            if iter.peek().is_some() || conf.loop_gif {
                //since picture height is in pixel, we divide by 2 to get the height in
                // terminal cells
                if let Err(e) = execute!(stdout(), cursor::MoveUp(print_height as u16)) {
                    if e.kind() == ErrorKind::BrokenPipe {
                        //Stop printing. Output is probably piped to `head` or a similar tool
                        break 'infinite;
                    } else {
                        return Err(e.into());
                    }
                }
            }
        }
        if !conf.loop_gif {
            break 'infinite;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_view_without_extension() {
        let conf = Config::test_config();
        let (tx, rx) = mpsc::channel();
        view_file(&conf, "img/bfa", (&tx, &rx)).unwrap();
    }
}

use image::{DynamicImage, GenericImageView, Pixel, Rgba};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const UPPER_HALF_BLOCK: &str = "\u{2580}";
const LOWER_HALF_BLOCK: &str = "\u{2584}";

pub fn print(img: &DynamicImage) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    let (width, _) = img.dimensions();

    let mut curr_row_px = 0;
    let mut curr_col_px = 0;
    let mut buffer: Vec<ColorSpec> = Vec::with_capacity(width as usize);
    let mut mode: Status = Status::TopRow;

    //iterate pixels and fill a buffer that contains 2 rows of pixels
    //2 rows translate to 1 row in the terminal by using half block, foreground and background
    //colors
    for pixel in img.pixels() {
        //if the alpha of the pixel is 0, print a predefined pixel based on the position in order
        //to mimic the chess board background
        let color = if is_pixel_transparent(pixel) {
            get_transparency_color(curr_row_px, curr_col_px)
        } else {
            get_color(get_pixel_data(pixel))
        };

        if mode == Status::TopRow {
            let mut c = ColorSpec::new();
            c.set_bg(Some(color));
            buffer.push(c);
        } else {
            let colorspec_to_upg = &mut buffer[curr_col_px as usize];
            colorspec_to_upg.set_fg(Some(color));
        }
        curr_col_px += 1;
        //if the buffer is full start adding the second row of pixels
        if is_buffer_full(&buffer, width) {
            if mode == Status::TopRow {
                mode = Status::BottomRow;
                curr_row_px += 1;
                curr_col_px = 0;
            }
            //only if the second row is completed flush the buffer and start again
            else if curr_col_px == width {
                curr_col_px = 0;
                curr_row_px += 1;
                print_buffer(&mut buffer, false);
                mode = Status::TopRow;
            }
        }
    }

    //buffer will be flushed if the image has an odd height
    if !buffer.is_empty() {
        print_buffer(&mut buffer, true);
    }

    clear_printer(&mut stdout);
}

fn print_buffer(buff: &mut Vec<ColorSpec>, is_flush: bool) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    let mut out_color;
    let mut out_char;
    let mut new_color;

    for c in buff.iter() {
        //if we need to flush it means that we must print only one row with UPPER_HALF_BLOCK
        //because it will be only the last row which contains 1 pixel
        if is_flush {
            new_color = ColorSpec::new();
            let bg = c.bg().unwrap();
            new_color.set_fg(Some(*bg));
            out_color = &new_color;
            out_char = UPPER_HALF_BLOCK;
        } else {
            out_color = c;
            out_char = LOWER_HALF_BLOCK;
        }
        change_stdout_color(&mut stdout, out_color);
        write!(stdout, "{}", out_char).unwrap_or_else(|_| handle_broken_pipe());
    }

    clear_printer(&mut stdout);
    write_newline(&mut stdout);
    buff.clear();
}

fn is_pixel_transparent(pixel: (u32, u32, Rgba<u8>)) -> bool {
    pixel.2.data[3] == 0
}

//TODO: some gifs do not specify every pixel in every frame (they reuse old pixels)
//experimenting is required to see how gifs like
//https://media.giphy.com/media/13gvXfEVlxQjDO/giphy.gif behave
fn get_transparency_color(row: u32, col: u32) -> Color {
    if row % 2 == col % 2 {
        Color::Rgb(102, 102, 102)
    } else {
        Color::Rgb(153, 153, 153)
    }
}

fn get_pixel_data<T: Pixel>(pixel: (u32, u32, T)) -> T {
    pixel.2
}

fn get_color(p: Rgba<u8>) -> Color {
    Color::Rgb(p.data[0], p.data[1], p.data[2])
}

fn is_buffer_full(buffer: &[ColorSpec], width: u32) -> bool {
    buffer.len() == width as usize
}

fn clear_printer(stdout: &mut StandardStream) {
    let c = ColorSpec::new();
    change_stdout_color(stdout, &c);
}

fn change_stdout_color(stdout: &mut StandardStream, color: &ColorSpec) {
    stdout
        .set_color(color)
        .unwrap_or_else(|_| handle_broken_pipe());
}

fn write_newline(stdout: &mut StandardStream) {
    writeln!(stdout).unwrap_or_else(|_| handle_broken_pipe());
}

//according to https://github.com/rust-lang/rust/issues/46016
fn handle_broken_pipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    };
}

//enum used to keep track where the current line of pixels processed should be displayed - as
//background or foreground color
#[derive(PartialEq)]
enum Status {
    TopRow,
    BottomRow,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_buffer_full() {
        let buffer = vec![ColorSpec::new(), ColorSpec::new()];
        let width = 2;
        assert!(is_buffer_full(&buffer, width));
    }
    #[test]
    fn test_print_buffer() {
        let mut buffer = vec![ColorSpec::new(), ColorSpec::new()];
        print_buffer(&mut buffer, false);
        assert!(buffer.len() == 0);
    }
    #[test]
    fn test_status_eq() {
        let s1 = Status::TopRow;
        let s2 = Status::BottomRow;
        assert!(s1 != s2);
    }
}

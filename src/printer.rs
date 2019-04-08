use image::{DynamicImage, GenericImageView};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub fn print(img: &DynamicImage) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let mut c = ColorSpec::new();

    let (width, height) = img.dimensions();

    const BLOCK: &str = "\u{2584}";
    let mut curr_row = 0;
    let mut curr_col = 0;
    let pixels = img.raw_pixels();
    let odd = height % 2 == 1;

    while curr_row < height / 2 {
        clear_printer(&mut c, &mut stdout);
        if curr_col == width {
            curr_row += 1;
            curr_col = 0;

            writeln!(&mut stdout, "")
                .unwrap_or_else(|e| eprintln!("Error while displaying image: {}", e));
        } else {
            let bg_start = (3 * (2 * curr_row * width + curr_col)) as usize;
            let fg_start = (3 * ((2 * curr_row + 1) * width + curr_col)) as usize;
            //println!("FG: {}, BG: {}", fg_start, bg_start);
            //println!("{}", start);
            c.set_fg(Some(Color::Rgb(
                pixels[fg_start],
                pixels[fg_start + 1],
                pixels[fg_start + 2],
            )))
            .set_bg(Some(Color::Rgb(
                pixels[bg_start],
                pixels[bg_start + 1],
                pixels[bg_start + 2],
            )))
            .set_bold(false);
            stdout
                .set_color(&c)
                .unwrap_or_else(|e| eprintln!("Error while changing terminal colors: {}", e));
            write!(&mut stdout, "{}", BLOCK)
                .unwrap_or_else(|e| eprintln!("Error while displaying image: {}", e));
            curr_col += 1;
        }
    }
    //check if there is a line of pixels that are not drawn yet
    if odd {
        curr_col = 0;
        while curr_col < width {
            clear_printer(&mut c, &mut stdout);
            let fg_start = (3 * (2 * curr_row * width + curr_col)) as usize;
            c.set_fg(Some(Color::Rgb(
                pixels[fg_start],
                pixels[fg_start + 1],
                pixels[fg_start + 2],
            )))
            .set_bold(false);
            stdout
                .set_color(&c)
                .unwrap_or_else(|e| eprintln!("Error while changing terminal colors: {}", e));
            write!(&mut stdout, "{}", "\u{2580}")
                .unwrap_or_else(|e| eprintln!("Error while displaying image: {}", e));
            curr_col += 1;
        }
        writeln!(&mut stdout, "")
            .unwrap_or_else(|e| eprintln!("Error while displaying image: {}", e));
    }
}

fn clear_printer(c: &mut ColorSpec, stdout: &mut StandardStream) {
    c.clear();
    stdout
        .set_color(&c)
        .unwrap_or_else(|e| eprintln!("Error while changing terminal colors: {}", e));
}

extern crate clap;
extern crate image;

use clap::{value_t, App, Arg};
use image::{FilterType, GenericImageView};

use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod size;

fn main() {
    let matches = App::new("Experiment")
        .version("1.0")
        .author("Atanas Yankov")
        .about("We will see what it does later on...")
        .arg(
            Arg::with_name("mirror")
                .short("m")
                .long("mirror")
                .help("Mirror the image"),
        )
        .arg(
            Arg::with_name("width")
                .short("w")
                .long("width")
                .takes_value(true)
                .help("Set the preferred width when displaying the image in the terminal"),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .takes_value(true)
                .help("Set the preferred height when displaying the image in the terminal"),
        )
        .arg(
            Arg::with_name("overwrite")
                .short("o")
                .long("overwrite")
                .help("Set whether the original file should be overwritten"),
        )
        .arg(
            Arg::with_name("FILE")
                .help("Set the image to manipulate")
                .required(true)
                .index(1),
        )
        .get_matches();

    let filename = matches.value_of("FILE").unwrap();
    let mut img = image::open(filename).unwrap();

    if matches.is_present("mirror") {
        img = img.fliph();
    }

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let mut c = ColorSpec::new();

    let mut counter = 0;
    let print_img;
    let mut modified_img;
    let (width, height) = img.dimensions();
    let (mut print_width, mut print_height) = img.dimensions();
    /*let chars = [
        "\u{2580}", "\u{2581}", "\u{2582}", "\u{2583}", "\u{2584}", "\u{2585}", "\u{2586}",
        "\u{2587}", "\u{2588}", "\u{2589}", "\u{258A}", "\u{258B}", "\u{258C}", "\u{258D}",
        "\u{258E}", "\u{258F}", "\u{2590}",
    ];*/
    let chars = ["\u{2584}"];

    let specified_width = matches.is_present("width");
    let specified_height = matches.is_present("height");

    if specified_width {
        let new_width = value_t!(matches, "width", u32).unwrap_or_else(|e| e.exit());
        print_width = new_width;
    }
    if specified_height {
        let new_height = value_t!(matches, "height", u32).unwrap_or_else(|e| e.exit());
        print_height = 2 * new_height;
    }
    if specified_width && specified_height {
        modified_img = img.thumbnail_exact(print_width, print_height);
        print_img = &modified_img;
    } else if specified_width || specified_height {
        modified_img = img.thumbnail(print_width, print_height);
        print_img = &modified_img;
    } else {
        match size::get_size() {
            Ok((w, h)) => {
                //only change values if the image needs to be resized (is bigger than the
                //terminal's size
                if width > w {
                    print_width = w;
                }
                if height > h {
                    print_height = 2 * h;
                }
            }
            Err(e) => {
                eprintln!("{}", e);
                //could not get terminal width => we fall back to a predefined value
                //maybe use env variable?
                print_width = 50;
            }
        };
        modified_img = img.resize(print_width, print_height, FilterType::Triangle);
        print_img = &modified_img;
    }
    for block in chars.iter() {
        println!("Trying with block: {}", block);
        let (width, height) = print_img.dimensions();
        let mut curr_row = 0;
        let mut curr_col = 0;
        let pixels = print_img.raw_pixels();
        println!("Size: {:?}", (width, height));
        let odd = height % 2 == 1;

        while curr_row < height / 2 {
            if curr_col == width {
                curr_row += 1;
                curr_col = 0;

                c.clear();
                stdout
                    .set_color(&c)
                    .unwrap_or_else(|e| eprintln!("Error while changing terminal colors: {}", e));

                writeln!(&mut stdout, "")
                    .unwrap_or_else(|e| eprintln!("Error while displaying image: {}", e));
            } else {
                c.clear();
                stdout.set_color(&c).unwrap();
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
                write!(&mut stdout, "{}", block)
                    .unwrap_or_else(|e| eprintln!("Error while displaying image: {}", e));
                curr_col += 1;
            }
        }
        //check if there is a line of pixels that are not drawn yet
        if odd {
            curr_col = 0;
            while curr_col < width {
                c.clear();
                stdout.set_color(&c).unwrap();
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

        //reset the color of stdout
        c.clear();
        stdout
            .set_color(&c)
            .unwrap_or_else(|e| eprintln!("Error while changing terminal colors: {}", e));
    }

    let (print_width, print_height) = print_img.dimensions();
    println!(
        "From {}x{} the image is now {}x{}",
        width, height, print_width, print_height
    );

    if matches.is_present("overwrite") {
        img.save(filename).expect("Failed saving image!");
    }
}

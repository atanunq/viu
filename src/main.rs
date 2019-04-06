extern crate clap;
extern crate image;

use clap::{value_t, App, Arg};
use image::GenericImageView;

use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

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
    let mut print_img = &img;
    let mut modified_img;
    let (width, height) = img.dimensions();
    let (mut print_width, mut print_height) = img.dimensions();
    let chars = [
        "\u{2580}", "\u{2581}", "\u{2582}", "\u{2583}", "\u{2584}", "\u{2585}", "\u{2586}",
        "\u{2587}", "\u{2588}", "\u{2589}", "\u{258A}", "\u{258B}", "\u{258C}", "\u{258D}",
        "\u{258E}", "\u{258F}", "\u{2590}",
    ];
    //let chars = ["\u{2589}"];

    let specified_width = matches.is_present("width");
    let specified_height = matches.is_present("height");

    if specified_width {
        let new_width = value_t!(matches, "width", u32).unwrap_or_else(|e| e.exit());
        print_width = new_width;
    }
    if specified_height {
        let new_height = value_t!(matches, "height", u32).unwrap_or_else(|e| e.exit());
        print_height = new_height;
    }
    if specified_width && specified_height {
        modified_img = img.thumbnail_exact(print_width, print_height);
        print_img = &modified_img;
    } else if specified_width || specified_height {
        modified_img = img.thumbnail(print_width, print_height);
        print_img = &modified_img;
    }
    for block in chars.iter() {
        println!("Trying with block: {}", block);
        let (width, _) = print_img.dimensions();
        for p in print_img.pixels() {
            counter = counter + 1;
            c.set_fg(Some(Color::Rgb(p.2[0], p.2[1], p.2[2])))
                .set_bold(false);
            stdout
                .set_color(&c)
                .unwrap_or_else(|e| eprintln!("Error while changing terminal colors: {}", e));
            write!(&mut stdout, "{}", block)
                .unwrap_or_else(|e| eprintln!("Error while displaying image: {}", e));
            if counter == width {
                writeln!(&mut stdout, "")
                    .unwrap_or_else(|e| eprintln!("Error while displaying image: {}", e));
                counter = 0;
            }
        }
        //reset the color of stdout
        c.clear();
        stdout
            .set_color(&c)
            .unwrap_or_else(|e| eprintln!("Error while changing terminal colors: {}", e));
    }

    println!(
        "From {}x{} the image is now {}x{}",
        width, height, print_width, print_height
    );
    if matches.is_present("overwrite") {
        img.save(filename).expect("Failed saving image!");
    }
}

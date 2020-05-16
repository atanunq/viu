use crate::term::truecolor_available;
use clap::{value_t, ArgMatches};

pub struct Config<'a> {
    pub files: Vec<&'a str>,
    pub loop_gif: bool,
    pub verbose: bool,
    pub name: bool,
    pub mirror: bool,
    pub transparent: bool,
    pub recursive: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub truecolor: bool,
    pub static_gif: bool,
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
        let static_gif = matches.is_present("static");
        let loop_gif = files.len() <= 1 && !once;
        let truecolor = truecolor_available();

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
            truecolor,
            static_gif,
        }
    }
    #[cfg(test)]
    pub fn test_config() -> Config<'a> {
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
            truecolor: true,
            static_gif: false,
        }
    }
}

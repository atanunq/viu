use crate::term::truecolor_available;
use clap::{value_t, ArgMatches};
use viuer::Config as ViuerConfig;

pub struct Config<'a> {
    pub files: Vec<&'a str>,
    pub loop_gif: bool,
    pub verbose: bool,
    pub name: bool,
    pub mirror: bool,
    pub recursive: bool,
    pub static_gif: bool,
    pub viuer_config: ViuerConfig,
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

        let transparent = matches.is_present("transparent");
        let truecolor = truecolor_available();

        let viuer_config = ViuerConfig {
            truecolor,
            transparent,
            width,
            height,
            ..Default::default()
        };

        Config {
            files,
            loop_gif,
            verbose: matches.is_present("verbose"),
            name: matches.is_present("name"),
            mirror: matches.is_present("mirror"),
            recursive: matches.is_present("recursive"),
            static_gif,
            viuer_config,
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
            recursive: false,
            static_gif: false,
            viuer_config: ViuerConfig::default(),
        }
    }
}

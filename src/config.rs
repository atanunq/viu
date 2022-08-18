use clap::ArgMatches;
use std::time::Duration;
use viuer::Config as ViuerConfig;

pub struct Config<'a> {
    pub files: Vec<&'a str>,
    pub loop_gif: bool,
    pub name: bool,
    pub recursive: bool,
    pub static_gif: bool,
    pub viuer_config: ViuerConfig,
    pub frame_duration: Option<Duration>,
}

impl<'a> Config<'a> {
    pub fn new(matches: &'a ArgMatches) -> Config<'a> {
        let width = if matches.is_present("width") {
            Some(matches.value_of_t_or_exit("width"))
        } else {
            None
        };
        let height = if matches.is_present("height") {
            Some(matches.value_of_t_or_exit("height"))
        } else {
            None
        };

        let x = if matches.is_present("x") {
            matches.value_of_t_or_exit("x")
        } else {
            0
        };

        let y = if matches.is_present("y") {
            matches.value_of_t_or_exit("y")
        } else {
            0
        };

        let files = match matches.values_of("FILE") {
            None => Vec::new(),
            Some(values) => values.collect(),
        };

        let once = matches.is_present("once");
        let static_gif = matches.is_present("static");
        let loop_gif = files.len() <= 1 && !once;

        let transparent = matches.is_present("transparent");

        let use_blocks = matches.is_present("blocks");

        let absolute_offset = matches.is_present("absolute-offset");

        let viuer_config = ViuerConfig {
            transparent,
            x,
            y,
            width,
            height,
            absolute_offset: absolute_offset,
            use_kitty: !use_blocks,
            use_iterm: !use_blocks,
            #[cfg(feature = "sixel")]
            use_sixel: !use_blocks,
            ..Default::default()
        };

        let frame_duration = if matches.is_present("frames-per-second") {
            let frame_rate: u16 = matches.value_of_t_or_exit("frames-per-second");
            Some(Duration::from_secs_f32(1.0 / frame_rate as f32))
        } else {
            None
        };
        Config {
            files,
            loop_gif,
            name: matches.is_present("name"),
            recursive: matches.is_present("recursive"),
            static_gif,
            viuer_config,
            frame_duration,
        }
    }
    #[cfg(test)]
    pub fn test_config() -> Config<'a> {
        Config {
            files: vec![],
            loop_gif: true,
            name: false,
            recursive: false,
            static_gif: false,
            viuer_config: ViuerConfig {
                absolute_offset: false,
                use_kitty: false,
                ..Default::default()
            },
            frame_duration: None,
        }
    }
}

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
        let width = matches.get_one("width").cloned();
        let height = matches.get_one("height").cloned();

        let files: Vec<&str> = matches
            .get_many::<String>("file")
            .unwrap_or_default()
            .map(|s| s.as_str())
            .collect();

        let absolute_offset = matches.get_flag("absolute-offset");
        let x: u16 = matches
            .get_one("x")
            .cloned()
            .expect("X offset must be present");
        let y: i16 = matches
            .get_one("y")
            .cloned()
            .expect("Y offset must be present");

        let use_blocks = matches.get_flag("blocks");
        let transparent = matches.get_flag("transparent");

        let viuer_config = ViuerConfig {
            width,
            height,
            x,
            y,
            transparent,
            absolute_offset,
            use_kitty: !use_blocks,
            use_iterm: !use_blocks,
            #[cfg(feature = "sixel")]
            use_sixel: !use_blocks,
            ..Default::default()
        };

        let frame_duration: Option<Duration> = matches
            .get_one::<u8>("frames-per-second")
            .cloned()
            .map(|f| Duration::from_secs_f32(1.0 / f as f32));

        let once = matches.get_flag("once");
        let static_gif = matches.get_flag("static");
        let loop_gif = files.len() <= 1 && !once;

        Config {
            files,
            loop_gif,
            name: matches.get_flag("name"),
            recursive: matches.get_flag("recursive"),
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

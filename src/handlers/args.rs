use clap::{builder::PossibleValue, Parser};

use crate::handlers::config::{Alignment, CompleteConfig, Palette, Theme};

impl clap::ValueEnum for Alignment {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Left, Self::Center, Self::Right]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }))
    }
}

impl clap::ValueEnum for Palette {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Pastel, Self::Vibrant, Self::Warm, Self::Cool]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::Pastel => "pastel",
            Self::Vibrant => "vibrant",
            Self::Warm => "warm",
            Self::Cool => "cool",
        }))
    }
}

impl clap::ValueEnum for Theme {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Dark, Self::Light]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::Light => "light",
            _ => "dark",
        }))
    }
}

#[derive(Parser, Debug)]
#[clap(rename_all = "kebab-case")]
#[clap(author, version, about)]
/// Twitch chat in the terminal
pub struct Cli {
    /// The streamer's name
    #[arg(short, long)]
    pub channel: Option<String>,
    /// File to log to
    #[arg(short, long)]
    pub log_file: Option<String>,
    /// If debug logs should be shown
    #[arg(short, long)]
    pub verbose: bool,
    /// The delay in milliseconds between terminal updates
    #[arg(short, long)]
    pub tick_delay: Option<u64>,
    /// The maximum amount of messages to be stored
    #[arg(short, long)]
    pub max_messages: Option<usize>,
    /// Show the date/time
    #[arg(short, long)]
    pub date_shown: bool,
    /// Maximum length for Twitch usernames
    #[arg(short = 'u', long)]
    pub max_username_length: Option<u16>,
    /// Username column alignment
    #[arg(short = 'a', long)]
    pub username_alignment: Option<Alignment>,
    /// Username color palette
    #[arg(short, long)]
    pub palette: Option<Palette>,
    /// Twitch badges support
    #[arg(short, long)]
    pub badges: bool,
    /// The theme of the terminal
    #[arg(long)]
    pub theme: Option<Theme>,
}

pub fn merge_args_into_config(config: &mut CompleteConfig, args: Cli) {
    // Terminal arguments
    if let Some(log_file) = args.log_file {
        config.terminal.log_file = Some(log_file);
    }
    config.terminal.verbose = args.verbose;

    if let Some(tick_delay) = args.tick_delay {
        config.terminal.tick_delay = tick_delay;
    }
    if let Some(max_messages) = args.max_messages {
        config.terminal.maximum_messages = max_messages;
    }

    // Twitch arguments
    if let Some(channel) = args.channel {
        config.twitch.channel = channel;
    }

    // Frontend arguments
    config.frontend.date_shown = args.date_shown;

    if let Some(maximum_username_length) = args.max_username_length {
        config.frontend.maximum_username_length = maximum_username_length;
    }
    if let Some(username_alignment) = args.username_alignment {
        config.frontend.username_alignment = username_alignment;
    }
    if let Some(palette) = args.palette {
        config.frontend.palette = palette;
    }
    config.frontend.badges = args.badges;
    if let Some(theme) = args.theme {
        config.frontend.theme = theme;
    }
}

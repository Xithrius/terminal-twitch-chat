use once_cell::sync::Lazy;

pub static HELP_COLUMN_TITLES: Lazy<Vec<&str>> =
    Lazy::new(|| vec!["State", "Keybind", "Description"]);

// TODO: Make this type have less complexity
#[allow(clippy::type_complexity)]
pub static HELP_KEYBINDS: Lazy<Vec<(&str, Vec<(&str, &str)>)>> = Lazy::new(|| {
    vec![
        (
            "Dashboard",
            vec![
                (
                    "Enter",
                    "Enter the channel that you've configured in the config",
                ),
                ("? or h", "Have the keybinds popup window appear"),
                ("q", "Quit the application"),
                ("s", "Open the recent channel search popup"),
                ("f", "Open the followed channel search popup"),
                ("Ctrl + p", "Manually crash the application"),
            ],
        ),
        (
            "Normal mode",
            vec![
                ("i or c", "Enter message (chat) mode for sending messages"),
                ("@", "Messaging mode with mention symbol"),
                ("/", "Messaging mode with command symbol"),
                ("? or h", "* You are here!"),
                ("q", "Quit the application"),
                ("s", "Open the recent channel search widget"),
                ("f", "Open the followed channel search widget"),
                ("S", "Go to the dashboard screen (start screen)"),
                ("Ctrl + f", "Search messages"),
                ("Ctrl + t", "Toggle the message filter"),
                ("Ctrl + r", "Reverse the message filter"),
                ("Ctrl + p", "Manually crash the application"),
                ("Esc", "Go back to the previous window"),
            ],
        ),
        (
            "Insert modes",
            vec![
                ("Tab", "Fill in suggestion, if available"),
                ("Enter", "Confirm the input text to go through"),
                ("Esc", "Go back to the previous window"),
                ("Ctrl + f", "Move cursor to the right"),
                ("Ctrl + b", "Move cursor to the left"),
                ("Ctrl + a", "Move cursor to the start"),
                ("Ctrl + e", "Move cursor to the end"),
                ("Ctrl + t", "Swap previous item with current item"),
                ("Ctrl + k", "Remove everything after the cursor"),
                ("Ctrl + u", "Remove everything before the cursor"),
                ("Ctrl + w", "Remove the previous word"),
                ("Ctrl + d", "Remove item to the right"),
                ("Ctrl + t", "Toggle the filter"),
                ("Ctrl + r", "Reverse the filter"),
                ("Alt + f", "Move to the end of the next word"),
                ("Alt + b", "Move to the start of the previous word"),
                ("Alt + t", "Swap previous word with current word"),
                ("Alt + e", "Toggle emote picker"),
            ],
        ),
    ]
});

// https://help.twitch.tv/s/article/chat-commands?language=en_US
pub static COMMANDS: Lazy<Vec<&str>> = Lazy::new(|| {
    vec![
        "ban",
        "unban",
        "clear",
        "color",
        "commercial",
        "delete",
        "disconnect",
        "emoteonly",
        "emoteonlyoff",
        "followers",
        "followersoff",
        "help",
        "host",
        "unhost",
        "marker",
        "me",
        "mod",
        "unmod",
        "mods",
        "r9kbeta",
        "r9kbetaoff",
        "raid",
        "unraid",
        "slow",
        "slowoff",
        "subscribers",
        "subscribersoff",
        "timeout",
        "untimeout",
        "vip",
        "unvip",
        "vips",
        "w",
    ]
});

pub static LINE_BUFFER_CAPACITY: usize = 4096;

// https://discuss.dev.twitch.tv/t/irc-bot-and-message-lengths/23327/4
pub static TWITCH_MESSAGE_LIMIT: usize = 500;

// https://www.reddit.com/r/Twitch/comments/32w5b2/username_requirements/
// This thread is from 8 years ago, so this regex match may be outdated.
// It is now possible to have channel names be 3 characters, such as "ppy".
pub static NAME_MAX_CHARACTERS: usize = 25;
pub static NAME_RESTRICTION_REGEX: Lazy<&str> = Lazy::new(|| "^[a-zA-Z0-9_]{3,25}$");

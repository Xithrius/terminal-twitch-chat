mod badges;
mod connection;

use std::collections::HashMap;

use flume::{Receiver, Sender};
use futures::StreamExt;
use irc::{
    client::prelude::Capability,
    proto::{Command, Message},
};
use log::{debug, info};

use crate::{
    handlers::{
        config::CompleteConfig,
        data::{Data, DataBuilder},
    },
    twitch::{
        badges::retrieve_user_badges,
        connection::{client_stream_reconnect, create_client_stream},
    },
};

#[derive(Debug)]
pub enum TwitchAction {
    Privmsg(String),
    Join(String),
}

pub async fn twitch_irc(mut config: CompleteConfig, tx: Sender<Data>, rx: Receiver<TwitchAction>) {
    info!("Spawned Twitch IRC thread.");

    let data_builder = DataBuilder::new(&config.frontend.date_format);
    let mut room_state_startup = false;

    let (mut client, mut stream) = create_client_stream(config.clone()).await;

    let sender = client.sender();

    // Request commands capabilities
    if client
        .send_cap_req(&[
            Capability::Custom("twitch.tv/commands"),
            Capability::Custom("twitch.tv/tags"),
        ])
        .is_err()
    {
        tx.send_async(
            data_builder.system(
                "Unable to request commands/tags capability, certain features may be affected."
                    .to_string(),
            ),
        )
        .await
        .unwrap();
    }

    loop {
        tokio::select! {
            biased;

            Ok(action) = rx.recv_async() => {
                let current_channel = format!("#{}", config.twitch.channel);

                match action {
                    TwitchAction::Privmsg(message) => {
                        debug!("Sending message to Twitch: {}", message);

                        client
                            .send_privmsg(current_channel, message)
                            .unwrap();
                    }
                    TwitchAction::Join(channel) => {
                        debug!("Switching to channel {}", channel);

                        let channel_list = format!("#{}", channel);

                        // Leave previous channel
                        if let Err(err) = sender.send_part(current_channel) {
                            tx.send_async(data_builder.twitch(err.to_string())).await.unwrap();
                        } else {
                            tx.send_async(data_builder.twitch(format!("Joined {}", channel_list))).await.unwrap();
                        }

                        // Join specified channel
                        if let Err(err) = sender.send_join(&channel_list) {
                            tx.send_async(data_builder.twitch(err.to_string())).await.unwrap();
                        }

                        // Set old channel to new channel
                        config.twitch.channel = channel;
                    }
                }
            }
            Some(message) = stream.next() => {
                match message {
                    Ok(message) => {
                        if let Some(b) = handle_message_command(message, tx.clone(), data_builder, config.frontend.badges, room_state_startup).await {
                            room_state_startup = b;
                        }
                    }
                    Err(err) => {
                        debug!("Twitch connection error encountered: {}, attempting to reconnect.", err);

                        client_stream_reconnect(err, tx.clone(), data_builder, &mut client, &mut stream, &config).await;
                    }
                }
            }
            else => {}
        };
    }
}

async fn handle_message_command(
    message: Message,
    tx: Sender<Data>,
    data_builder: DataBuilder<'_>,
    badges: bool,
    room_state_startup: bool,
) -> Option<bool> {
    let mut tags: HashMap<&str, &str> = HashMap::new();

    if let Some(ref ref_tags) = message.tags {
        for tag in ref_tags {
            if let Some(ref tag_value) = tag.1 {
                tags.insert(&tag.0, tag_value);
            }
        }
    }

    match message.command {
        Command::PRIVMSG(ref _target, ref msg) => {
            // lowercase username from message
            let mut name = message.source_nickname().unwrap().to_string();

            if badges {
                retrieve_user_badges(&mut name, &message);
            }

            tx.send_async(DataBuilder::user(name.to_string(), msg.to_string()))
                .await
                .unwrap();

            debug!("Message received from twitch: {} - {}", name, msg);
        }
        Command::NOTICE(ref _target, ref msg) => {
            tx.send_async(data_builder.twitch(msg.to_string()))
                .await
                .unwrap();
        }
        Command::Raw(ref cmd, ref _items) => {
            match cmd.as_ref() {
                "ROOMSTATE" => {
                    // Only display roomstate on startup, since twitch
                    // sends a NOTICE whenever roomstate changes.
                    if !room_state_startup {
                        handle_roomstate(&tx, &tags).await;
                    }

                    return Some(true);
                }
                "USERNOTICE" => {
                    if let Some(value) = tags.get("system-msg") {
                        tx.send_async(data_builder.twitch((*value).to_string()))
                            .await
                            .unwrap();
                    }
                }
                _ => (),
            }
        }
        _ => (),
    }

    None
}

pub async fn handle_roomstate(tx: &Sender<Data>, tags: &HashMap<&str, &str>) {
    let mut room_state = String::new();

    for (name, value) in tags.iter() {
        match *name {
            "emote-only" if *value == "1" => {
                room_state.push_str("The channel is emote-only.\n");
            }
            "followers-only" if *value != "-1" => {
                room_state.push_str("The channel is followers-only.\n");
            }
            "subs-only" if *value == "1" => {
                room_state.push_str("The channel is subscribers-only.\n");
            }
            "slow" if *value != "0" => {
                room_state.push_str("The channel has a ");
                room_state.push_str(value);
                room_state.push_str("s slowmode.\n");
            }
            _ => (),
        }
    }

    // Trim last newline
    room_state.pop();

    if room_state.is_empty() {
        return;
    }

    tx.send_async(DataBuilder::user(String::from("Info"), room_state))
        .await
        .unwrap();
}

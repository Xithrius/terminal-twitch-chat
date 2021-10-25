use futures::StreamExt;
use irc::{
    client::{data, prelude::*, Client},
    proto::Command,
};
use std::collections::HashMap;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::handlers::{
    config::CompleteConfig,
    data::{Data, DataBuilder},
};

pub async fn twitch_irc(config: &CompleteConfig, tx: Sender<Data>, mut rx: Receiver<String>) {
    let irc_config = data::Config {
        nickname: Some(config.twitch.username.to_owned()),
        server: Some(config.twitch.server.to_owned()),
        channels: vec![format!("#{}", config.twitch.channel)],
        password: Some(config.twitch.token.to_owned()),
        port: Some(6667),
        use_tls: Some(false),
        ..Default::default()
    };

    let mut client = Client::from_config(irc_config.clone()).await.unwrap();
    client.identify().unwrap();
    let mut stream = client.stream().unwrap();
    let data_builder = DataBuilder::new(&config.frontend.date_format);
    let mut room_state_startup = false;

    // Request commands capabilities
    if client
        .send_cap_req(&[
            Capability::Custom("twitch.tv/commands"),
            Capability::Custom("twitch.tv/tags"),
        ])
        .is_err()
    {
        tx.send(
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

            Some(message) = rx.recv() => {
                client
                .send_privmsg(format!("#{}", config.twitch.channel), message)
                .unwrap();
            }
            Some(_message) = stream.next() => {
                let message = _message.unwrap();
                let mut tags: HashMap<&str, &str> = std::collections::HashMap::new();
                if let Some(ref _tags) = message.tags {
                    for tag in _tags {
                        if let Some(ref tag_value) = tag.1 {
                            tags.insert(&tag.0, tag_value);
                        }
                    }
                }

                match message.command {
                    Command::PRIVMSG(ref _target, ref msg) => {
                        // lowercase username from message
                        let mut name = match message.source_nickname() {
                            Some(username) => username.to_string(),
                            None => "Undefined username".to_string(),
                        };
                        // try to get username from message tags
                        if let Some(ref tags) = message.tags {
                            for tag in tags {
                                if tag.0 == *"display-name" {
                                    if let Some(ref value) = tag.1 {
                                        name = value.to_string();
                                    }
                                    break;
                                }
                            }
                        }
                        tx.send(data_builder.user(name, msg.to_string()))
                        .await
                        .unwrap();
                    }
                    Command::NOTICE(ref _target, ref msg) => {
                        tx.send(data_builder.twitch(msg.to_string()))
                        .await
                        .unwrap();
                    }
                    Command::Raw(ref cmd, ref _items) => {
                        match cmd.as_ref() {
                            "ROOMSTATE" => {
                                // Only display roomstate on startup, since twitch
                                // sends a NOTICE whenever roomstate changes.
                                if !room_state_startup {
                                    handle_roomstate(&tx, data_builder, &tags).await;
                                }
                                room_state_startup = true;
                            }
                            "USERNOTICE" => {
                                if let Some(value) = tags.get("system-msg") {
                                    tx.send(data_builder.twitch(value.to_string()))
                                    .await
                                    .unwrap();
                                }
                            }
                            _ => ()
                        }
                    }
                    _ => ()
                }
            }
        };
    }
}

pub async fn handle_roomstate(
    tx: &Sender<Data>,
    builder: DataBuilder<'_>,
    tags: &HashMap<&str, &str>,
) {
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
    if !room_state.is_empty() {
        tx.send(builder.user(String::from("Info"), room_state))
            .await
            .unwrap();
    }
}

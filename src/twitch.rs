use std::panic::panic_any;

use anyhow::Result;
use chrono::offset::Local;
use futures::{FutureExt, StreamExt};
use irc::{
    client::{data, Client},
    proto::Command,
};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::unconstrained,
};

use crate::handlers::{config::CompleteConfig, data::Data};

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

    let mut client = Client::from_config(irc_config)
        .await
        .unwrap_or_else(|err| panic_any(err));

    client.identify().unwrap_or_else(|err| panic_any(err));

    let mut stream = client.stream().unwrap_or_else(|err| panic_any(err));

    loop {
        if let Some(Some(Ok(message))) = unconstrained(stream.next()).now_or_never() {
            if let Command::PRIVMSG(ref _target, ref msg) = message.command {
                let user = match message.source_nickname() {
                    Some(username) => username.to_string(),
                    None => "Undefined username".to_string(),
                };
                tx.send(Data::new(
                    Local::now()
                        .format(config.frontend.date_format.as_str())
                        .to_string(),
                    user,
                    msg.to_string(),
                    false,
                ))
                .await
                .unwrap();
            }
        }

        if let Some(Some(message)) = unconstrained(rx.recv()).now_or_never() {
            client
                .send_privmsg(format!("#{}", config.twitch.channel), message)
                .unwrap();
        }
    }
}

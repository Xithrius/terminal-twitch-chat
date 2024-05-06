use std::{
    fmt::Display,
    string::{String, ToString},
    vec::Vec,
};

use color_eyre::Result;
use reqwest::Client;
use serde::Deserialize;

use crate::{handlers::config::TwitchConfig, ui::components::utils::SearchItemGetter};

use super::oauth::{get_channel_id, get_twitch_client};

const FOLLOWER_COUNT: usize = 100;

#[derive(Deserialize, Debug, Clone, Default)]
#[allow(dead_code)]
pub struct FollowingUser {
    broadcaster_id: String,
    pub broadcaster_login: String,
    pub broadcaster_name: String,
    followed_at: String,
}

impl Display for FollowingUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.broadcaster_login)
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
#[allow(dead_code)]
struct Pagination {
    cursor: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[allow(dead_code)]
pub struct FollowingList {
    pub total: u64,
    pub data: Vec<FollowingUser>,
    pagination: Pagination,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Following {
    // TODO: Don't re-create client on new requests
    // client: &Client,
    twitch_config: TwitchConfig,
    list: FollowingList,
}

// https://dev.twitch.tv/docs/api/reference/#get-followed-channels
pub async fn get_user_following(client: &Client, user_id: i32) -> Result<FollowingList> {
    Ok(client
        .get(format!(
            "https://api.twitch.tv/helix/channels/followed?user_id={user_id}&first={FOLLOWER_COUNT}",
        ))
        .send()
        .await?
        .error_for_status()?
        .json::<FollowingList>()
        .await?)
}

pub async fn get_following(twitch_config: &TwitchConfig) -> Result<FollowingList> {
    let oauth_token = twitch_config.token.clone();
    let app_user = twitch_config.username.clone();

    let client = get_twitch_client(oauth_token).await.unwrap();

    let user_id = get_channel_id(&client, &app_user).await.unwrap();

    get_user_following(&client, user_id).await
}

impl Following {
    pub fn new(twitch_config: TwitchConfig) -> Self {
        Self {
            twitch_config,
            list: FollowingList::default(),
        }
    }
}

impl SearchItemGetter<String> for Following {
    async fn get_items(&mut self) -> Result<Vec<String>> {
        let following = get_following(&self.twitch_config).await;

        following.map(|v| {
            v.data
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        })
    }
}

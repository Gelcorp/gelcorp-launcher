use reqwest::Client;
use serde::{ Deserialize, Serialize };

use crate::StdError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerProfile {
  pub id: String,
  pub name: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub skins: Vec<Skin>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub capes: Vec<Cape>,
}

impl PlayerProfile {
  pub async fn get(client: &Client, mojang_token: &str) -> Result<PlayerProfile, StdError> {
    client
      .get("https://api.minecraftservices.com/minecraft/profile")
      .bearer_auth(mojang_token)
      .send().await?
      .error_for_status()?
      .json().await
      .map_err(|err| format!("Failed to get deserialize profile: {}", err).into())
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum State {
  Active,
  Inactive,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum SkinType {
  SLIM,
  CLASSIC,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cape {
  pub alias: String,
  pub id: String,
  pub state: State,
  pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Skin {
  pub id: String,
  pub state: State,
  pub texture_key: String,
  pub url: String,
  pub variant: Option<SkinType>,
}

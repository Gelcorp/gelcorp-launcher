use chrono::Utc;
use log::info;
use minecraft_launcher_core::bootstrap::auth::UserAuthentication;
use minecraft_msa_auth::MinecraftAuthorizationFlow;
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

use crate::{ app::error::StdError, config::mojang_api_helper::PlayerProfile, msa_auth::MSAuthToken };

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum Authentication {
  Msa(MsaMojangAuth),
  Offline {
    username: String,
    uuid: String,
  },
}

impl TryInto<UserAuthentication> for Authentication {
  type Error = StdError;
  fn try_into(self) -> Result<UserAuthentication, Self::Error> {
    match self {
      Authentication::Msa(msa) => {
        Ok(UserAuthentication {
          username: msa.username,
          uuid: Uuid::parse_str(&msa.uuid)?,
          access_token: Some(msa.moj_token),
        })
      }
      Authentication::Offline { username, .. } => Ok(UserAuthentication::offline(&username)),
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct MsaMojangAuth {
  username: String,
  uuid: String,

  moj_token: String,
  moj_expiration_date: i64,

  msa_access_token: String,
  msa_refresh_token: String,
  msa_expiration_date: i64,
}

impl MsaMojangAuth {
  pub(crate) fn expired_msa(&self) -> bool {
    Utc::now().timestamp_millis() > self.msa_expiration_date
  }

  pub(crate) async fn refresh(&mut self, force: bool) -> Result<(), StdError> {
    if self.expired_msa() || force {
      self.refresh_msa(force).await?;
    }
    if self.expired_mojang() || force {
      self.refresh_mojang(force).await?;
    }
    self.refresh_profile().await?;
    Ok(())
  }

  pub(crate) async fn refresh_msa(&mut self, force: bool) -> Result<(), StdError> {
    if !self.expired_msa() && !force {
      return Ok(());
    }
    info!("Refreshing Microsoft token...");
    let mut msa = MSAuthToken {
      access_token: self.msa_access_token.clone(),
      refresh_token: self.msa_refresh_token.clone(),
      expiration_date: self.msa_expiration_date,
    };
    msa.refresh(force).await?;
    self.msa_access_token = msa.access_token;
    self.msa_refresh_token = msa.refresh_token;
    self.msa_expiration_date = msa.expiration_date;
    Ok(())
  }

  pub(crate) fn expired_mojang(&self) -> bool {
    Utc::now().timestamp_millis() > self.moj_expiration_date
  }

  pub(crate) async fn refresh_mojang(&mut self, force: bool) -> Result<(), StdError> {
    if !self.expired_mojang() && !force {
      return Ok(());
    }
    info!("Refreshing Minecraft token...");
    let mc_token = MinecraftAuthorizationFlow::new(Client::new())
      .exchange_microsoft_token(&self.msa_access_token).await
      .map_err(|err| format!("Failed to exchange msa token: {}", err))?;

    self.username.clone_from(mc_token.username());
    self.moj_token = mc_token.access_token().clone().into_inner();
    self.moj_expiration_date = Utc::now().timestamp_millis() + (mc_token.expires_in() as i64) * 1000;
    Ok(())
  }

  pub(crate) async fn refresh_profile(&mut self) -> Result<(), StdError> {
    info!("Fetching profile info...");
    let PlayerProfile { id, name, .. } = PlayerProfile::get(&Client::new(), &self.moj_token).await?;
    self.username = name;
    self.uuid = id;
    info!("Username = {} UUID = {}", self.username, self.uuid);
    Ok(())
  }

  pub(crate) async fn from(msa: MSAuthToken) -> Result<Self, StdError> {
    let mut new = Self {
      username: String::new(),
      uuid: String::new(),

      moj_token: String::new(),
      moj_expiration_date: 0,

      msa_access_token: msa.access_token,
      msa_refresh_token: msa.refresh_token,
      msa_expiration_date: msa.expiration_date,
    };
    new.refresh(false).await?;
    Ok(new)
  }
}

impl From<MsaMojangAuth> for UserAuthentication {
  fn from(moj_auth: MsaMojangAuth) -> Self {
    UserAuthentication {
      username: moj_auth.username,
      uuid: Uuid::parse_str(&moj_auth.uuid).unwrap(),
      access_token: Some(moj_auth.moj_token),
    }
  }
}

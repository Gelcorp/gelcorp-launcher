use std::{ fs::{ create_dir_all, File }, path::PathBuf };

use chrono::Utc;
use log::{ error, info };
use minecraft_launcher_core::profile_manager::auth::{ CommonUserAuthentication, OfflineUserAuthentication, UserAuthentication };
use minecraft_msa_auth::MinecraftAuthorizationFlow;
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use tauri::Window;
use uuid::Uuid;

use crate::{ msa_auth::MSAuthToken, StdError, GAME_DIR_PATH };

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct LauncherConfig {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) authentication: Option<Authentication>,

  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub(crate) selected_options: Vec<String>,
  #[serde(default = "LauncherConfig::default_providers", skip_serializing_if = "Vec::is_empty")]
  pub(crate) providers: Vec<String>,

  #[serde(default = "LauncherConfig::default_memory_max")]
  pub(crate) memory_max: u16,
}

impl Default for LauncherConfig {
  fn default() -> Self {
    Self {
      authentication: None,
      selected_options: vec![],
      providers: LauncherConfig::default_providers(),
      memory_max: LauncherConfig::default_memory_max(),
    }
  }
}

impl LauncherConfig {
  pub(crate) fn get_file_path() -> PathBuf {
    GAME_DIR_PATH.join("launcher_config.json")
  }

  pub(crate) async fn load_from_file() -> Self {
    let mut config: LauncherConfig = File::open(Self::get_file_path())
      .ok()
      .and_then(|mut f| serde_json::from_reader(&mut f).ok())
      .unwrap_or_default();
    config.validate_session().await;
    let _ = config.save_to_file();
    config
  }

  pub(crate) async fn validate_session(&mut self) {
    if let Some(authentication) = self.authentication.as_mut() {
      if let Authentication::Msa(msa) = authentication {
        if let Err(err) = msa.refresh(false).await {
          error!("Failed to refresh msa token: {}", err);
          self.authentication = None;
          let _ = self.save_to_file();
        } else {
          info!("Logged in successfully to msa");
        }
      }
    }
  }

  pub(crate) fn save_to_file(&self) -> Result<(), StdError> {
    let path = Self::get_file_path();
    let mut file = File::create(&path)?;
    if let Some(parent) = path.parent() {
      create_dir_all(parent)?;
    }
    serde_json::to_writer_pretty(&mut file, &self)?;
    Ok(())
  }

  pub(crate) fn broadcast_update(&self, window: &Window) -> Result<(), StdError> {
    window.emit("launcher_config_update", &self)?;
    Ok(())
  }

  fn default_memory_max() -> u16 {
    1024
  }

  fn default_providers() -> Vec<String> {
    env!("DEFAULT_PROVIDERS")
      .split(" ")
      .map(|s| s.to_string())
      .collect()
  }
}

// Authentication

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Authentication {
  Msa(MsaMojangAuth),
  Offline {
    username: String,
    uuid: String,
  },
}

impl TryInto<Box<dyn UserAuthentication + Send + Sync>> for Authentication {
  type Error = StdError;
  fn try_into(self) -> Result<Box<dyn UserAuthentication + Send + Sync>, Self::Error> {
    match self {
      Authentication::Msa(msa) => {
        let auth: CommonUserAuthentication = msa.into();
        Ok(Box::new(auth))
      }
      Authentication::Offline { username, uuid } => Ok(Box::new(OfflineUserAuthentication { username, uuid: Uuid::parse_str(&uuid)? })),
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MsaMojangAuth {
  username: String,
  uuid: String,

  moj_token: String,
  moj_expiration_date: i64,

  msa_access_token: String,
  msa_refresh_token: String,
  msa_expiration_date: i64,
}

impl MsaMojangAuth {
  fn expired_msa(&self) -> bool {
    Utc::now().timestamp_millis() > self.msa_expiration_date
  }

  async fn refresh(&mut self, force: bool) -> Result<(), StdError> {
    if self.expired_msa() || force {
      self.refresh_msa(force).await?;
    }
    if self.expired_mojang() || force {
      self.refresh_mojang(force).await?;
    }
    self.refresh_profile().await?;
    Ok(())
  }

  async fn refresh_msa(&mut self, force: bool) -> Result<(), StdError> {
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

  fn expired_mojang(&self) -> bool {
    Utc::now().timestamp_millis() > self.moj_expiration_date
  }

  async fn refresh_mojang(&mut self, force: bool) -> Result<(), StdError> {
    if !self.expired_mojang() && !force {
      return Ok(());
    }
    info!("Refreshing Minecraft token...");
    let mc_token = MinecraftAuthorizationFlow::new(Client::new())
      .exchange_microsoft_token(&self.msa_access_token).await
      .map_err(|err| format!("Failed to exchange msa token: {}", err))?;

    self.username = mc_token.username().clone();
    self.moj_token = mc_token.access_token().clone().into_inner();
    self.moj_expiration_date = Utc::now().timestamp_millis() + (mc_token.expires_in() as i64) * 1000;
    Ok(())
  }

  async fn refresh_profile(&mut self) -> Result<(), StdError> {
    info!("Fetching profile info...");
    let response = Client::new()
      .get("https://api.minecraftservices.com/minecraft/profile")
      .bearer_auth(&self.moj_token)
      .send().await?
      .error_for_status()?
      .json::<Value>().await?;
    self.username = response["name"].as_str().ok_or("Failed to get username")?.to_string();
    self.uuid = response["id"].as_str().ok_or("Failed to get uuid")?.to_string();
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

impl Into<CommonUserAuthentication> for MsaMojangAuth {
  fn into(self) -> CommonUserAuthentication {
    CommonUserAuthentication {
      access_token: self.msa_access_token,
      auth_playername: self.username,
      auth_uuid: Uuid::parse_str(&self.uuid).unwrap(),
      user_type: "msa".to_string(),
    }
  }
}

pub mod auth;
mod mojang_api_helper;

use std::{ fs::{ create_dir_all, File }, path::PathBuf };

use log::{ error, info };
use serde::{ Deserialize, Serialize };
use tauri::Window;

use crate::{ StdError, GAME_DIR_PATH };

use self::auth::Authentication;

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
    if let Some(Authentication::Msa(msa)) = self.authentication.as_mut() {
      if let Err(err) = msa.refresh(false).await {
        error!("Failed to refresh msa token: {}", err);
        self.authentication = None;
        let _ = self.save_to_file();
      } else {
        info!("Logged in successfully to msa");
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
    1536
  }

  fn default_providers() -> Vec<String> {
    env!("DEFAULT_PROVIDERS")
      .split(' ')
      .map(|s| s.to_string())
      .collect()
  }
}

// Authentication

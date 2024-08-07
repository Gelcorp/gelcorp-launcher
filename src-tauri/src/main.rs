#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Prevents additional console window on Windows in release, DO NOT REMOVE!!

pub mod app;
pub mod constants;

mod config;
mod logger;
mod java;
mod modpack_downloader;
mod log_flusher;
mod forge;

use app::{ game_status::GameStatusState, state::LauncherState };
use config::LauncherConfig;
use constants::{ LAUNCHER_DIRECTORY, UPDATE_ENDPOINTS };
use log::info;
use serde::Serialize;
use tauri::utils::config::UpdaterEndpoint;

use tokio::sync::Mutex;

use crate::{ log_flusher::LAUNCHER_LOGS, logger::{ setup_logger, LauncherAppender }, modpack_downloader::{ ModpackDownloader, ModpackProvider } };

#[derive(Default, Serialize, Clone)]
pub struct DownloadProgress {
  pub status: String,
  pub current: usize,
  pub total: usize,
}

#[tokio::main]
async fn main() {
  let logs_dir = LAUNCHER_DIRECTORY.join("logs").join("gelcorp-launcher");
  setup_logger(&logs_dir).expect("Failed to initialize logger");
  info!("Starting tauri application...");

  LauncherAppender::add_callback(
    Box::new(move |msg| {
      LAUNCHER_LOGS.log(msg.trim_end());
      Ok(())
    })
  );

  let launcher_config = LauncherConfig::load_from_file().await;
  let providers: Vec<ModpackProvider> = launcher_config.providers
    .iter()
    .map(|s| ModpackProvider::new(s))
    .collect();
  let modpack_downloader = ModpackDownloader::new(LAUNCHER_DIRECTORY.clone(), providers);

  let launcher_state = LauncherState {
    launcher_config: Mutex::new(launcher_config),
    modpack_downloader: Mutex::new(modpack_downloader),
    game_status: GameStatusState::new(),
  };

  let update_endpoints = UPDATE_ENDPOINTS.split(' ')
    .map(|s| s.parse().expect("Failed to parse update endpoint"))
    .map(UpdaterEndpoint)
    .collect();

  app::gui::init(launcher_state, update_endpoints);
}

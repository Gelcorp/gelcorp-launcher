use std::sync::Arc;

use log::error;
use sysinfo::System;
use tauri::{ utils::config::UpdaterEndpoint, Builder, Manager, State, Window };
use uuid::Uuid;

use crate::{
  config::{ auth::{ Authentication, MsaMojangAuth }, LauncherConfig },
  constants::{ LAUNCHER_NAME, LAUNCHER_VERSION },
  log_flusher::{ self, flush_all_logs },
  modpack_downloader::ModpackInfo,
  msa_auth,
};

use super::{ error::LauncherError, game::real_start_game, game_status::GameStatus, state::LauncherState };

#[tauri::command]
async fn fetch_modpack_info(state: State<'_, LauncherState>) -> Result<ModpackInfo, LauncherError> {
  let mut downloader = state.modpack_downloader.lock().await;
  let modpack_info = downloader.get_or_fetch_modpack_info().await?;
  Ok(modpack_info.clone())
}

#[tauri::command]
fn get_system_memory() -> u64 {
  System::new_all().total_memory()
}

#[tauri::command]
async fn start_game(state: State<'_, LauncherState>, window: Window) -> Result<(), LauncherError> where Window: Sync {
  let window = Arc::new(window);
  let res = real_start_game(state.clone(), window.clone()).await.map_err(|e| e.into());
  flush_all_logs(&window.app_handle());
  if let Err(err) = &res {
    error!("Failed to start game: {}", err);
  }
  state.game_status.set(GameStatus::Idle);
  res
}

#[tauri::command]
fn get_game_status(state: State<'_, LauncherState>) -> GameStatus {
  state.game_status.get()
}

#[tauri::command]
async fn get_launcher_config(state: State<'_, LauncherState>) -> Result<LauncherConfig, LauncherError> {
  Ok(state.launcher_config.lock().await.clone())
}

#[tauri::command]
async fn set_launcher_config(state: State<'_, LauncherState>, config: LauncherConfig) -> Result<(), LauncherError> {
  let mut state = state.launcher_config.lock().await;
  *state = config;
  state.save_to_file()?;
  Ok(())
}

#[tauri::command]
async fn login_offline(state: State<'_, LauncherState>, window: Window, username: String) -> Result<(), LauncherError> {
  let mut state = state.launcher_config.lock().await;
  let uuid = Uuid::new_v3(&Uuid::NAMESPACE_DNS, format!("OfflinePlayer:{username}").as_bytes()).to_string();
  state.authentication.replace(Authentication::Offline { username, uuid });
  state.broadcast_update(&window)?;
  state.save_to_file()?;
  Ok(())
}

#[tauri::command]
async fn login_msa(state: State<'_, LauncherState>, window: Window) -> Result<(), LauncherError> {
  let ms_auth_token = msa_auth::get_msa_token(&window).await.map_err(|err| LauncherError::Other(format!("Failed to get msa token: {}", err)))?;
  let auth = MsaMojangAuth::from(ms_auth_token).await.map_err(|err| LauncherError::Other(format!("Failed to login: {}", err)))?;

  let mut state = state.launcher_config.lock().await;
  state.authentication.replace(Authentication::Msa(auth));
  state.broadcast_update(&window)?;
  state.save_to_file()?;
  Ok(())
}

pub fn init(launcher_state: LauncherState, update_endpoints: Vec<UpdaterEndpoint>) {
  let title = format!("{} {}", LAUNCHER_NAME, LAUNCHER_VERSION);

  let mut context = tauri::generate_context!();
  context.config_mut().tauri.updater.endpoints.replace(update_endpoints);

  let app = Builder::default()
    .plugin(log_flusher::init())
    .invoke_handler(
      tauri::generate_handler![
        start_game,
        get_launcher_config,
        set_launcher_config,
        login_offline,
        login_msa,
        fetch_modpack_info,
        get_system_memory,
        get_game_status
      ]
    )
    .build(context)
    .expect("error while building tauri application");

  let main_win = app.get_window("main").expect("failed to get main window");
  let _ = main_win.set_title(&title);

  launcher_state.game_status.set_window(main_win);
  app.manage(launcher_state);
  app.run(|_, _| {})
}

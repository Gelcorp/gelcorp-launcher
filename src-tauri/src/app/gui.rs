use std::collections::HashMap;

use log::error;
use minecraft_launcher_core::bootstrap::auth::UserAuthentication;
use sysinfo::System;
use tauri::{ Builder, Manager, State, Url, WebviewWindow };

use crate::{
  config::{ auth::{ Authentication, MsaMojangAuth }, LauncherConfig },
  constants::{ LAUNCHER_NAME, LAUNCHER_VERSION, G1GC_JRE_FLAGS, ZGC_JRE_FLAGS },
  log_flusher::{ self, flush_all_logs },
  modpack_downloader::ModpackInfo,
};

use super::{ error::LauncherError, game, game_status::GameStatus, msa_auth, state::LauncherState };

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
fn get_default_jre_flags() -> HashMap<String, String> {
  let mut flags = HashMap::new();
  flags.insert("g1gc".to_owned(), G1GC_JRE_FLAGS.to_owned());
  flags.insert("zgc".to_owned(), ZGC_JRE_FLAGS.to_owned());
  flags
}

#[tauri::command]
async fn start_game(state: State<'_, LauncherState>, window: WebviewWindow) -> Result<(), LauncherError> where WebviewWindow: Sync {
  let res = game::launch_game(&state, &window).await.map_err(|e| e.into());
  flush_all_logs(window.app_handle());
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
async fn login_offline(state: State<'_, LauncherState>, window: WebviewWindow, username: String) -> Result<(), LauncherError> {
  let mut state = state.launcher_config.lock().await;
  let auth = UserAuthentication::offline(&username);
  state.authentication.replace(Authentication::Offline { username, uuid: auth.uuid });
  state.broadcast_update(&window)?;
  state.save_to_file()?;
  Ok(())
}

#[tauri::command]
async fn login_msa(state: State<'_, LauncherState>, window: WebviewWindow) -> Result<(), LauncherError> {
  let ms_auth_token = msa_auth
    ::show_microsoft_prompt(&window).await
    .map_err(|err| LauncherError::Other(format!("Failed to get msa token: {}", err)))?;
  let auth = MsaMojangAuth::from(ms_auth_token).await.map_err(|err| LauncherError::Other(format!("Failed to login: {}", err)))?;

  let mut state = state.launcher_config.lock().await;
  state.authentication.replace(Authentication::Msa(auth));
  state.broadcast_update(&window)?;
  state.save_to_file()?;
  Ok(())
}

pub async fn init(launcher_state: LauncherState, update_endpoints: Vec<Url>) -> anyhow::Result<()> {
  let title = format!("{} {}", LAUNCHER_NAME, LAUNCHER_VERSION);

  let mut context = tauri::generate_context!();
  let endpoints = context
    .config_mut()
    .plugins.0.get_mut("updater")
    .and_then(|config| config.get_mut("endpoints"))
    .unwrap();
  *endpoints = serde_json::to_value(update_endpoints)?;

  let app = Builder::default()
    .plugin(log_flusher::init())
    .plugin(tauri_plugin_updater::Builder::new().build())
    .setup(move |app| {
      let main_win = app.get_webview_window("main").expect("failed to get main window");
      let _ = main_win.set_title(&title);

      launcher_state.game_status.set_window(main_win);
      app.manage(launcher_state);

      Ok(())
    })
    .invoke_handler(
      tauri::generate_handler![
        start_game,
        get_launcher_config,
        set_launcher_config,
        login_offline,
        login_msa,
        fetch_modpack_info,
        get_system_memory,
        get_default_jre_flags,
        get_game_status
      ]
    )
    .build(context)
    .expect("error while building tauri application");

  app.run(|_, _| {});

  Ok(())
}

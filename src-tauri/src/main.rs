#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Prevents additional console window on Windows in release, DO NOT REMOVE!!

mod msa_auth;
mod config;
mod logger;
mod java;
mod modpack_downloader;
mod game_status;
mod log_flusher;
mod forge;

use std::{ env, fs, io::BufRead, path::{ Path, PathBuf }, sync::Arc };

use config::{ auth::{ Authentication, MsaMojangAuth }, LauncherConfig };
use game_status::{ GameStatus, GameStatusState };
use log::{ debug, error, info, warn };
use log_flusher::flush_all_logs;
use minecraft_launcher_core::{
  bootstrap::{ auth::UserAuthentication, options::{ GameOptionsBuilder, LauncherOptions }, GameBootstrap },
  json::MCVersion,
  version_manager::{ downloader::progress::{ CallbackReporter, Event, ProgressReporter }, VersionManager },
};
use modpack_downloader::ModpackInfo;
use once_cell::sync::Lazy;
use regex::{ Captures, Regex };
use serde::Serialize;
use sysinfo::System;
use tauri::{ utils::config::UpdaterEndpoint, Builder, Manager, State, Window };

use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
  java::{ check_java_dir, download_java },
  log_flusher::{ GAME_LOGS, LAUNCHER_LOGS },
  logger::{ setup_logger, LauncherAppender },
  modpack_downloader::{ ModpackDownloader, ModpackProvider },
};

static GAME_STATUS_STATE: Lazy<GameStatusState> = Lazy::new(GameStatusState::new);

const LAUNCHER_NAME: &str = env!("LAUNCHER_NAME");
const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
const GAME_DIR_PATH: Lazy<PathBuf> = Lazy::new(|| resolve_path(env!("GAME_DIR_PATH")));

type StdError = Box<dyn std::error::Error>;

#[derive(Debug, Error)]
enum TauriError {
  #[error(transparent)] Reqwest(#[from] reqwest::Error),
  #[error(transparent)] Io(#[from] std::io::Error),
  #[error("{0}")] Other(String),
}

impl From<StdError> for TauriError {
  fn from(error: StdError) -> Self {
    Self::Other(error.to_string())
  }
}

impl Serialize for TauriError {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
    serializer.serialize_str(&self.to_string())
  }
}

#[tauri::command]
async fn fetch_modpack_info(modpack_downloader: State<'_, Mutex<ModpackDownloader>>) -> Result<ModpackInfo, TauriError> {
  let mut downloader = modpack_downloader.lock().await;
  let modpack_info = downloader.get_or_fetch_modpack_info().await?;
  Ok(modpack_info.clone())
}

#[tauri::command]
fn get_system_memory() -> u64 {
  System::new_all().total_memory()
}

#[tauri::command]
async fn start_game(
  state: State<'_, Mutex<LauncherConfig>>,
  modpack_downloader: State<'_, Mutex<ModpackDownloader>>,
  window: Window
) -> Result<(), TauriError>
  where Window: Sync
{
  let window = Arc::new(window);
  let res = real_start_game(state, modpack_downloader, window.clone()).await.map_err(|e| e.into());
  flush_all_logs(&window.app_handle());
  if let Err(err) = &res {
    error!("Failed to start game: {}", err);
  }
  GAME_STATUS_STATE.set(GameStatus::Idle);
  res
}

#[derive(Default, Serialize, Clone)]
pub struct DownloadProgress {
  pub status: String,
  pub current: usize,
  pub total: usize,
}

async fn real_start_game(
  state: State<'_, Mutex<LauncherConfig>>,
  modpack_downloader: State<'_, Mutex<ModpackDownloader>>,
  window: Arc<Window>
) -> Result<(), StdError>
  where Window: Sync
{
  let authentication = {
    let config = &*state.lock().await;
    let authentication = config.authentication.as_ref();
    if authentication.is_none() {
      config.broadcast_update(&window)?;
      return Err("Not logged in!".into());
    }
    authentication.unwrap().clone()
  };

  let reporter: ProgressReporter = {
    let window = Arc::clone(&window);
    let progress = std::sync::Mutex::new(None::<DownloadProgress>);
    Arc::new(
      CallbackReporter::new(move |event| {
        let progress = &mut *progress.lock().unwrap();
        let mut new_progress = progress.clone().unwrap_or_default();
        let done = matches!(event, Event::Done);
        match event {
          Event::Status(status) => {
            new_progress.status = status;
          }
          Event::Progress(current) => {
            new_progress.current = current;
          }
          Event::Total(total) => {
            new_progress.total = total;
          }
          Event::Setup { status, total } => {
            new_progress = DownloadProgress { status, current: 0, total: total.unwrap_or(0) };
          }
          _ => {}
        }
        if done {
          progress.take();
        } else {
          progress.replace(new_progress);
        }
        let _ = window.emit("update_progress", progress.clone());
      })
    )
  };

  info!("Attempting to launch the game...");
  let mc_dir = GAME_DIR_PATH.clone();
  let java_path = mc_dir.join("jre-runtime");
  let java_executable_path = java_path.join("bin").join("java.exe");

  GAME_STATUS_STATE.set(GameStatus::Downloading);
  debug!("Checking java runtime...");
  if !check_java_dir(&java_path) {
    info!("Java runtime not found. Downloading...");
    download_java(reporter.clone(), &java_path, "17").await.map_err(|err| TauriError::Other(format!("Failed to download java: {}", err)))?;
    info!("Java downloaded successfully!");
  }

  let mut downloader = modpack_downloader.lock().await;
  {
    debug!("Checking modpack...");
    let selected_options = state.lock().await.selected_options.clone();
    downloader.download_and_install(reporter.clone(), selected_options).await?;
  }

  let ModpackInfo { minecraft_version, forge_version, .. } = downloader.get_or_fetch_modpack_info().await?;
  let (forge_installer_path, forge_version_name) = forge::check_forge(&mc_dir, minecraft_version, forge_version, &java_executable_path).await?;
  info!("Forge Version: {}", &forge_version_name);

  let auth: UserAuthentication = authentication.try_into()?;
  info!("Logged in as {}", auth.username);

  let jvm_args = format!(
    "-Xms512M -Xmx{}M -Dforgewrapper.librariesDir={} -Dforgewrapper.installer={} -Dforgewrapper.minecraft={} -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M",
    state.lock().await.memory_max,
    mc_dir.join("libraries").display(),
    forge_installer_path.display(),
    mc_dir.join(format!("versions/{id}/{id}.jar", id = &forge_version_name)).display()
  );
  let jvm_args: Vec<String> = jvm_args
    .split(' ')
    .map(|s| s.to_string())
    .collect();

  let mc_version = MCVersion::from(forge_version_name);
  let natives_dir = mc_dir.join("natives");

  if fs::remove_dir_all(&natives_dir).is_err() {
    warn!("Couldn't cleanup natives directory");
  }

  let game_opts = GameOptionsBuilder::default()
    .game_dir(mc_dir)
    .java_path(java_executable_path)
    .launcher_options(LauncherOptions::new(LAUNCHER_NAME, LAUNCHER_VERSION))
    .authentication(auth)
    .jvm_args(jvm_args)
    .natives_dir(natives_dir)
    .build()
    .map_err(|err| TauriError::Other(format!("Failed to create game options: {err}")))?;
  let env_features = game_opts.env_features();

  reporter.setup("Fetching version manifest", Some(2));
  let mut version_manager = VersionManager::load(&game_opts.game_dir, &env_features, None).await?;

  info!("Queuing library & version downloads");
  reporter.status("Resolving local version");
  reporter.progress(1);
  let manifest = version_manager.resolve_local_version(&mc_version, true, false).await?;
  if !manifest.applies_to_current_environment(&env_features) {
    return Err(format!("Version {} is is incompatible with the current environment", mc_version).into());
  }
  reporter.done();

  version_manager.download_required_files(&manifest, &reporter, None, None).await?;

  let mut process = GameBootstrap::new(game_opts)
    .launch_game(&manifest)
    .map_err(|err| TauriError::Other(format!("Failed to launch the game: {err}")))?;

  GAME_STATUS_STATE.set(GameStatus::Playing);
  loop {
    let mut buf = String::new();
    if let Ok(length) = process.stdout().read_line(&mut buf) {
      if length > 0 {
        println!("{}", &buf.trim_end());
        GAME_LOGS.log(buf.trim_end());
        buf.clear();
      }
    }

    if !process.stderr().buffer().is_empty() {
      if let Ok(length) = process.stderr().read_line(&mut buf) {
        if length > 0 {
          println!("{}", &buf.trim_end());
          GAME_LOGS.log(buf.trim_end());
          buf.clear();
        }
      }
    }

    if let Some(exit_status) = process.exit_status() {
      if exit_status == 0 {
        info!("Game exited successfully");
        break Ok(());
      } else {
        info!("Game exited with code {exit_status}");
        break Err(format!("Failed to launch the game. Process exited with code {exit_status}").into());
      }
    }
  }
}

#[tauri::command]
fn get_game_status() -> GameStatus {
  GAME_STATUS_STATE.get()
}

#[tauri::command]
async fn get_launcher_config(state: State<'_, Mutex<LauncherConfig>>) -> Result<LauncherConfig, TauriError> {
  Ok(state.lock().await.clone())
}

#[tauri::command]
async fn set_launcher_config(state: State<'_, Mutex<LauncherConfig>>, config: LauncherConfig) -> Result<(), TauriError> {
  let mut state = state.lock().await;
  *state = config;
  state.save_to_file()?;
  Ok(())
}

#[tauri::command]
async fn login_offline(state: State<'_, Mutex<LauncherConfig>>, window: Window, username: String) -> Result<(), TauriError> {
  let mut state = state.lock().await;
  let uuid = Uuid::new_v3(&Uuid::NAMESPACE_DNS, format!("OfflinePlayer:{username}").as_bytes()).to_string();
  state.authentication.replace(Authentication::Offline { username, uuid });
  state.broadcast_update(&window)?;
  state.save_to_file()?;
  Ok(())
}

#[tauri::command]
async fn login_msa(state: State<'_, Mutex<LauncherConfig>>, window: Window) -> Result<(), TauriError> {
  let ms_auth_token = msa_auth::get_msa_token(&window).await.map_err(|err| TauriError::Other(format!("Failed to get msa token: {}", err)))?;
  let auth = MsaMojangAuth::from(ms_auth_token).await.map_err(|err| TauriError::Other(format!("Failed to login: {}", err)))?;

  let mut state = state.lock().await;
  state.authentication.replace(Authentication::Msa(auth));
  state.broadcast_update(&window)?;
  state.save_to_file()?;
  Ok(())
}

#[tokio::main]
async fn main() {
  let logs_dir = GAME_DIR_PATH.join("logs/gelcorp-launcher");
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

  let mut context = tauri::generate_context!();
  let update_endpoints = {
    let endpoints = env!("UPDATE_ENDPOINTS").split(' ');
    endpoints.map(|s| UpdaterEndpoint(s.parse().expect("Failed to parse update endpoint"))).collect::<Vec<_>>()
  };
  context.config_mut().tauri.updater.endpoints.replace(update_endpoints);

  let title = format!("{} {}", LAUNCHER_NAME, LAUNCHER_VERSION);

  Builder::default()
    .setup(move |app| {
      let win = app.get_window("main").unwrap();
      let _ = win.set_title(&title);
      GAME_STATUS_STATE.set_window(win);
      Ok(())
    })
    .plugin(log_flusher::init())
    .manage(Mutex::new(launcher_config))
    .manage(Mutex::new(ModpackDownloader::new(GAME_DIR_PATH.to_path_buf(), providers)))
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
    .run(context)
    .expect("error while running tauri application");
}

fn resolve_path(path: impl AsRef<Path>) -> PathBuf {
  let path = path.as_ref();
  let regex = Regex::new(r"%([a-zA-Z0-9]+)%").unwrap();
  let mut new_path_buf = PathBuf::new();

  for component in path.components() {
    if let Some(component_str) = component.as_os_str().to_str() {
      let replaced_component = regex
        .replace_all(component_str, |captures: &Captures| {
          match env::var(&captures[1]) {
            Ok(value) => value,
            Err(_) => captures[0].to_string(),
          }
        })
        .as_ref()
        .to_string();

      new_path_buf.push(Path::new(&replaced_component));
    } else {
      new_path_buf.push(component);
    }
  }

  new_path_buf
}

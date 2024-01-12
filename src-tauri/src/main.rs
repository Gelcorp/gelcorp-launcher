#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Prevents additional console window on Windows in release, DO NOT REMOVE!!

mod msa_auth;
mod logger;
mod java;
mod modpack_downloader;

use std::{
  path::{ PathBuf, Path },
  env::{ self, temp_dir, current_dir },
  io::BufRead,
  sync::{ Arc, Mutex },
  thread::{ self, sleep },
  time::Duration,
  fs,
};

use forge_downloader::{ forge_client_install::ForgeClientInstall, download_utils::forge::ForgeVersionHandler };
use log::{ info, debug };
use minecraft_launcher_core::{
  options::{ GameOptionsBuilder, LauncherOptions },
  versions::info::MCVersion,
  profile_manager::auth::{ CommonUserAuthentication, UserAuthentication },
  MinecraftGameRunner,
};
use minecraft_msa_auth::MinecraftAuthorizationFlow;
use once_cell::sync::Lazy;
use regex::{ Captures, Regex };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use tauri::{ Window, Manager, Builder };

use thiserror::Error;

use crate::{ logger::{ LauncherAppender, setup_logger }, java::{ download_java, check_java_dir } };

static LAUNCHER_LOGS: Lazy<Arc<Mutex<Vec<String>>>> = Lazy::new(|| Arc::new(Mutex::new(vec![])));
static GAME_LOGS: Lazy<Arc<Mutex<Vec<String>>>> = Lazy::new(|| Arc::new(Mutex::new(vec![])));

const LAUNCHER_NAME: &str = "Survitroll Launcher";
const LAUNCHER_VERSION: &str = "2.0.0";
const MINECRAFT_FORGE_VERSION: (&str, &str) = ("1.20.1", "47.2.0");

/* TODO:
    - Add a command to login to Cracked/Msa account
    - Check for the auth to be valid at the start of the program
*/

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authentication {
  Offline {
    username: String,
  },
  Msa {
    msa_token: Option<String>,
    msa_refresh_token: Option<String>,
    msa_token_expire: Option<i32>,
  },
}

#[derive(Default, Serialize, Deserialize)]
struct LauncherConfig {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  authentication: Option<Authentication>,
  #[serde(default = "LauncherConfig::default_memory_max")]
  memory_max: u16,
}

impl LauncherConfig {
  fn load_from_file() -> Self {
    // TODO: load from file, otherwise return None
    let mut config = Self {
      authentication: None,
      memory_max: LauncherConfig::default_memory_max(),
    };
    config.validate_session();
    config
  }

  fn validate_session(&mut self) {
    // TODO: check if expired, and then clear the auth method (only on ram)
  }

  fn save_to_file(&self) {
    // TODO: save to file
  }

  fn update_state(&self, window: &Window) {}

  fn default_memory_max() -> u16 {
    1024
  }
}

#[derive(Debug, Error)]
enum TauriError {
  #[error(transparent)] Reqwest(#[from] reqwest::Error),
  #[error(transparent)] Io(#[from] std::io::Error),
  #[error("{0}")] Other(String),
}

impl From<Box<dyn std::error::Error>> for TauriError {
  fn from(error: Box<dyn std::error::Error>) -> Self {
    Self::Other(error.to_string())
  }
}

impl Serialize for TauriError {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
    serializer.serialize_str(&self.to_string())
  }
}

#[tauri::command]
async fn start_game(state: tauri::State<'_, LauncherConfig>, window: Window, mc_dir: String) -> Result<(), TauriError> where Window: Sync {
  let window = Arc::new(window);
  info!("Attempting to launch the game...");

  let mc_dir = sanitize_path_buf(PathBuf::from(mc_dir));
  let java_path = mc_dir.join("jre-runtime");
  let java_executable_path = java_path.join("bin").join("java.exe");
  info!(" \\--Game Version: {:?}", &MINECRAFT_FORGE_VERSION);
  info!(" \\--MC Directory: {}", &mc_dir.to_str().unwrap());
  info!(" \\--Java Path: {}", &java_path.to_str().unwrap());

  debug!("Checking java runtime...");
  if !check_java_dir(&java_path) {
    info!("Java runtime not found. Downloading...");
    download_java(&java_path, "17").await.map_err(|err| TauriError::Other(format!("Failed to download java: {}", err)))?;
    info!("Java downloaded successfully!");
  }

  let forge_version_name = check_forge(&mc_dir, &java_executable_path).await?;
  info!("Forge Version: {}", &forge_version_name);

  info!("Logging in to MSA...");
  let ms_auth_token = msa_auth::get_msa_token(&window).await.map_err(|err| TauriError::Other(format!("Failed to get msa token: {}", err)))?;

  let mc_token = MinecraftAuthorizationFlow::new(Client::new())
    .exchange_microsoft_token(ms_auth_token.access_token).await
    .map_err(|err| TauriError::Other(format!("Failed to exchange msa token: {}", err)))?;
  let mc_token = mc_token.access_token().clone().into_inner();

  let auth = CommonUserAuthentication::from_minecraft_token(&mc_token).await.unwrap();
  info!("Logged in as {}", auth.auth_player_name());

  let mem_max = format!("-Xmx{}M", state.memory_max);
  let jvm_args: Vec<&str> = vec![
    "-Xms512M",
    &mem_max,
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+UseG1GC",
    "-XX:G1NewSizePercent=20",
    "-XX:G1ReservePercent=20",
    "-XX:MaxGCPauseMillis=50",
    "-XX:G1HeapRegionSize=32M"
  ];
  let jvm_args: Vec<String> = jvm_args
    .iter()
    .map(|s| s.to_string())
    .collect();

  let game_opts = GameOptionsBuilder::default()
    .game_dir(mc_dir)
    .java_path(java_executable_path)
    .version(MCVersion::from(forge_version_name))
    .launcher_options(LauncherOptions::new(LAUNCHER_NAME, LAUNCHER_VERSION))
    .authentication(Box::new(auth))
    .jvm_args(jvm_args)
    .build()
    .map_err(|err| TauriError::Other(format!("Failed to create game options: {err}")))?;

  let mut process = MinecraftGameRunner::new(game_opts)
    .launch().await
    .map_err(|err| TauriError::Other(format!("Failed to launch the game: {err}")))?;

  loop {
    if let Some(exit_status) = process.exit_status() {
      if exit_status == 0 {
        info!("Game exited successfully");
        break Ok(());
      } else {
        info!("Game exited with code {exit_status}");
        break Err(TauriError::Other(format!("Failed to launch the game. Process exited with code {exit_status}")));
      }
    }

    let mut buf = String::new();
    if let Ok(length) = process.stdout().read_line(&mut buf) {
      if length > 0 {
        println!("{}", &buf.trim());
        GAME_LOGS.lock().unwrap().push(buf.trim().to_string());
        buf.clear();
      }
    }

    if process.stderr().buffer().len() > 0 {
      if let Ok(length) = process.stderr().read_line(&mut buf) {
        if length > 0 {
          println!("{}", &buf.trim());
          GAME_LOGS.lock().unwrap().push(buf.trim().to_string());
          buf.clear();
        }
      }
    }
  }

  // TODO: implement a similar aproach with a monitor
  // bootstrap.on_client_event({
  //   let window = Arc::clone(&window);
  //   move |event| {
  //     println!("{event:?}");
  //     window.emit("client_event", event).unwrap();
  //   }
  // });
}

fn main() {
  // TODO: use game dir
  setup_logger(&current_dir().unwrap().join("launcher_logs")).expect("Failed to initialize logger");
  info!("Starting tauri application...");
  LauncherAppender::add_callback(
    Box::new(move |msg| {
      LAUNCHER_LOGS.lock()?.push(msg.trim().to_string());
      Ok(())
    })
  );

  Builder::default()
    .setup(|app| {
      let win = app.get_window("main").unwrap();
      let builder = thread::Builder::new();
      builder
        .name("launcher-log-watcher".to_owned())
        .spawn(move || {
          loop {
            if let Ok(mut logs) = LAUNCHER_LOGS.try_lock() {
              if !logs.is_empty() {
                win.emit("launcher_log", logs.clone()).unwrap();
                logs.clear();
              }
            }
            if let Ok(mut logs) = GAME_LOGS.try_lock() {
              if !logs.is_empty() {
                win.emit("log", logs.clone()).unwrap();
                logs.clear();
              }
            }
            sleep(Duration::from_millis(5));
          }
        })
        .expect("Failed to spawn log watcher thread");
      Ok(())
    })
    .manage(LauncherConfig::load_from_file())
    .invoke_handler(tauri::generate_handler![start_game])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

async fn check_forge(mc_dir: &PathBuf, java_path: &PathBuf) -> Result<String, TauriError> {
  let versions_dir = mc_dir.join("versions");
  if versions_dir.is_dir() {
    let forge_folder_re = Regex::new(&format!("{}.+{}", MINECRAFT_FORGE_VERSION.0, MINECRAFT_FORGE_VERSION.1)).unwrap();
    let forge_version_name = versions_dir
      .read_dir()?
      .into_iter()
      .filter_map(|dir| dir.ok())
      .filter_map(|dir| dir.file_name().into_string().ok())
      .find(|name| forge_folder_re.is_match(name));
    if let Some(name) = forge_version_name {
      return Ok(name);
    }
  }

  let forge_version_handler = ForgeVersionHandler::new().await?;
  let forge_version = forge_version_handler.get_by_forge_version(&MINECRAFT_FORGE_VERSION.1).expect("Failed to get forge version");
  let forge_installer_path = {
    let response = Client::new().get(&forge_version.get_installer_url()).send().await?.error_for_status()?;
    let forge_installer = temp_dir().join("forge-installer.jar");
    fs::write(&forge_installer, &response.bytes().await?)?;
    forge_installer
  };
  let mut forge_installer_info = ForgeClientInstall::new(forge_installer_path, java_path.clone())?;
  let forge_version_id = forge_installer_info.get_profile().get_version_id();

  if !mc_dir.join("versions").join(&forge_version_id).join(format!("{}.json", forge_version_id)).is_file() {
    info!("Forge not installed! Setting up forge...");
    forge_installer_info.install_forge(&mc_dir, |_| true).await?;
    info!("Forge installed!");
  }
  Ok(forge_version_id)
}

fn sanitize_path_buf(path: PathBuf) -> PathBuf {
  let regex = regex::Regex::new(r"%([a-zA-Z0-9]+)%").unwrap();
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

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Prevents additional console window on Windows in release, DO NOT REMOVE!!

mod msa_auth;
mod logger;
mod java;
mod modpack_downloader;

use std::{ path::{ PathBuf, Path }, io::BufRead, sync::{ Arc, Mutex }, thread::{ self, sleep }, time::Duration, fs::{ self, File, create_dir_all }, env };

use forge_downloader::{
  forge_client_install::ForgeClientInstall,
  download_utils::forge::ForgeVersionHandler,
  forge_installer_profile::ForgeVersionInfo,
};
use log::{ info, debug, error };
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
use serde_json::json;
use tauri::{ Window, Manager, Builder, State };

use thiserror::Error;

use crate::{ logger::{ LauncherAppender, setup_logger }, java::{ download_java, check_java_dir } };

static LAUNCHER_LOGS: Lazy<Arc<Mutex<Vec<String>>>> = Lazy::new(|| Arc::new(Mutex::new(vec![])));
static GAME_LOGS: Lazy<Arc<Mutex<Vec<String>>>> = Lazy::new(|| Arc::new(Mutex::new(vec![])));

const LAUNCHER_NAME: &str = "Survitroll Launcher";
const LAUNCHER_VERSION: &str = "2.0.0";
const GAME_DIR_PATH: Lazy<PathBuf> = Lazy::new(|| resolve_path("%appdata%/.minecraft_test_rust"));

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
async fn start_game(state: State<'_, LauncherConfig>, window: Window) -> Result<(), TauriError> where Window: Sync {
  let window = Arc::new(window);
  let res = real_start_game(state, window.clone()).await.map_err(|e| e.into());
  flush_launcher_logs(&window);
  if let Err(err) = &res {
    error!("Failed to start game: {}", err);
  }
  res
}

async fn real_start_game(state: State<'_, LauncherConfig>, window: Arc<Window>) -> Result<(), StdError> where Window: Sync {
  info!("Attempting to launch the game...");
  let mc_dir = GAME_DIR_PATH.clone();
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

  let (forge_installer_path, forge_version_name) = check_forge(&mc_dir, &java_executable_path).await?;
  info!("Forge Version: {}", &forge_version_name);

  info!("Logging in to MSA...");
  let ms_auth_token = msa_auth::get_msa_token(&window).await.map_err(|err| TauriError::Other(format!("Failed to get msa token: {}", err)))?;

  let mc_token = MinecraftAuthorizationFlow::new(Client::new())
    .exchange_microsoft_token(ms_auth_token.access_token).await
    .map_err(|err| TauriError::Other(format!("Failed to exchange msa token: {}", err)))?;
  let mc_token = mc_token.access_token().clone().into_inner();

  let auth = CommonUserAuthentication::from_minecraft_token(&mc_token).await.unwrap();
  // let auth = OfflineUserAuthentication::new("Player");
  info!("Logged in as {}", auth.auth_player_name());

  let jvm_args = format!(
    "-Xms512M -Xmx{}M -Dforgewrapper.librariesDir={} -Dforgewrapper.installer={} -Dforgewrapper.minecraft={} -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M",
    state.memory_max,
    mc_dir.join("libraries").display(),
    forge_installer_path.display(),
    mc_dir.join(format!("versions/{id}/{id}.jar", id = &forge_version_name)).display()
  );
  let jvm_args: Vec<String> = jvm_args
    .split(" ")
    .map(|s| s.to_string())
    .collect();

  let game_opts = GameOptionsBuilder::default()
    .game_dir(mc_dir)
    .java_path(java_executable_path)
    .version(MCVersion::from(forge_version_name))
    .launcher_options(LauncherOptions::new(LAUNCHER_NAME, LAUNCHER_VERSION))
    .authentication(Box::new(auth))
    .jvm_args(jvm_args)
    .max_concurrent_downloads(4)
    .max_download_attempts(15)
    .build()
    .map_err(|err| TauriError::Other(format!("Failed to create game options: {err}")))?;

  let mut process = MinecraftGameRunner::new(game_opts)
    .launch().await
    .map_err(|err| TauriError::Other(format!("Failed to launch the game: {err}")))?;

  loop {
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

  // TODO: implement a similar aproach with a monitor
  // bootstrap.on_client_event({
  //   let window = Arc::clone(&window);
  //   move |event| {
  //     println!("{event:?}");
  //     window.emit("client_event", event).unwrap();
  //   }
  // });
}

fn flush_launcher_logs(win: &Window) {
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
}

fn main() {
  let logs_dir = GAME_DIR_PATH.join("logs/gelcorp-launcher");
  setup_logger(&logs_dir).expect("Failed to initialize logger");
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
            flush_launcher_logs(&win);
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

async fn check_forge(mc_dir: &PathBuf, java_path: &PathBuf) -> Result<(PathBuf, String), TauriError> {
  let versions_dir = mc_dir.join("versions");

  // Fetch version
  let version_handler = ForgeVersionHandler::new().await?;
  let version_info = version_handler.get_by_forge_version(&MINECRAFT_FORGE_VERSION.1).expect("Failed to get forge version");
  let installer_path = version_info.get_artifact().get_local_path(&mc_dir.join("libraries"));

  if versions_dir.is_dir() && installer_path.is_file() {
    let forge_folder_re = Regex::new(&format!("{}.+{}", MINECRAFT_FORGE_VERSION.0, MINECRAFT_FORGE_VERSION.1)).unwrap();
    let forge_version_name = versions_dir
      .read_dir()?
      .into_iter()
      .filter_map(|dir| dir.ok())
      .filter_map(|dir| dir.file_name().into_string().ok())
      .find(|name| forge_folder_re.is_match(name));
    if let Some(name) = forge_version_name {
      return Ok((installer_path, name));
    }
  }

  // Download installer if needed
  if !installer_path.is_file() {
    if let Some(parent) = installer_path.parent() {
      create_dir_all(parent)?;
    }

    let bytes = Client::new().get(&version_info.get_installer_url()).send().await?.error_for_status()?.bytes().await?;
    fs::write(&installer_path, &bytes)?;
  }

  // Open installer
  let mut install_handler = ForgeClientInstall::new(installer_path.clone(), java_path.clone())?;
  let forge_version_id = install_handler.get_profile().get_version_id();

  let forge_version_path = mc_dir.join(format!("versions/{id}/{id}.json", id = &forge_version_id));
  if !forge_version_path.is_file() {
    info!("Forge not installed! Setting up forge...");
    install_handler.install_forge(&mc_dir, |_| true).await?;
    debug!("Setting up forge wrapper...");
    setup_forge_wrapper(&forge_version_path)?;
    info!("Forge installed!");
  }
  Ok((installer_path, forge_version_id))
}

fn setup_forge_wrapper(forge_version_path: &PathBuf) -> Result<(), StdError> {
  let mut version_info: ForgeVersionInfo = serde_json::from_reader(File::open(forge_version_path)?)?;
  let wrapper_lib =
    json!({
    "downloads": {
      "artifact": {
        "sha1": "155ac9f4e5f65288eaacae19025ac4d9da1f0ef2",
        "size": 34910,
        "url": "https://github.com/ZekerZhayard/ForgeWrapper/releases/download/1.5.7/ForgeWrapper-1.5.7.jar"
      }
    },
    "name": "io.github.zekerzhayard:ForgeWrapper:1.5.7"
  });
  version_info.libraries.push(serde_json::from_value(wrapper_lib)?);
  version_info.main_class = "io.github.zekerzhayard.forgewrapper.installer.Main".to_string();
  serde_json::to_writer_pretty(&mut File::create(forge_version_path)?, &version_info)?;
  Ok(())
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

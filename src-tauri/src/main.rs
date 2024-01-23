#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Prevents additional console window on Windows in release, DO NOT REMOVE!!

mod msa_auth;
mod logger;
mod java;
mod modpack_downloader;

use std::{
  path::{ PathBuf, Path },
  io::BufRead,
  sync::{ Arc, Mutex },
  thread::{ self, sleep },
  time::Duration,
  fs::{ self, File, create_dir_all },
  env,
  ops::DerefMut,
  collections::VecDeque,
};

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
static LAUNCHER_LOGS_CACHE: Lazy<Arc<Mutex<VecDeque<String>>>> = Lazy::new(|| Arc::new(Mutex::new(VecDeque::new())));
static GAME_LOGS: Lazy<Arc<Mutex<Vec<String>>>> = Lazy::new(|| Arc::new(Mutex::new(vec![])));

const LAUNCHER_NAME: &str = "Survitroll Launcher";
const LAUNCHER_VERSION: &str = "2.0.0";
const GAME_DIR_PATH: Lazy<PathBuf> = Lazy::new(|| resolve_path("%appdata%/.minecraft_test_rust"));

const MINECRAFT_FORGE_VERSION: (&str, &str) = ("1.20.1", "47.2.0");

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Authentication {
  Offline {
    username: String,
    uuid: String,
  },
  Msa(MsaMojangAuth),
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

  async fn from(msa: MSAuthToken) -> Result<Self, StdError> {
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

#[derive(Serialize, Deserialize, Clone)]
struct LauncherConfig {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  authentication: Option<Authentication>,
  #[serde(default = "LauncherConfig::default_memory_max")]
  memory_max: u16,
}

impl Default for LauncherConfig {
  fn default() -> Self {
    Self {
      authentication: None,
      memory_max: LauncherConfig::default_memory_max(),
    }
  }
}

impl LauncherConfig {
  fn get_file_path() -> PathBuf {
    GAME_DIR_PATH.join("launcher_config.json")
  }

  async fn load_from_file() -> Self {
    let mut config: LauncherConfig = File::open(Self::get_file_path())
      .ok()
      .and_then(|mut f| serde_json::from_reader(&mut f).ok())
      .unwrap_or_default();
    config.validate_session().await;
    let _ = config.save_to_file();
    config
  }

  async fn validate_session(&mut self) {
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

  fn save_to_file(&self) -> Result<(), StdError> {
    let path = Self::get_file_path();
    let mut file = File::create(&path)?;
    if let Some(parent) = path.parent() {
      create_dir_all(parent)?;
    }
    serde_json::to_writer_pretty(&mut file, &self)?;
    Ok(())
  }

  fn broadcast_update(&self, window: &Window) -> Result<(), StdError> {
    window.emit("launcher_config_update", &self)?;
    Ok(())
  }

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
async fn start_game(state: State<'_, Mutex<LauncherConfig>>, window: Window) -> Result<(), TauriError> where Window: Sync {
  let window = Arc::new(window);
  let res = real_start_game(state, window.clone()).await.map_err(|e| e.into());
  flush_launcher_logs(&window);
  if let Err(err) = &res {
    error!("Failed to start game: {}", err);
  }
  res
}

async fn real_start_game(state: State<'_, Mutex<LauncherConfig>>, window: Arc<Window>) -> Result<(), StdError> where Window: Sync {
  let authentication = {
    let config = state.lock().unwrap();
    let authentication = config.authentication.as_ref();
    if authentication.is_none() {
      config.broadcast_update(&window)?;
      return Err("Not logged in!".into());
    }
    authentication.unwrap().clone()
  };
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

  let auth: Box<dyn UserAuthentication + Send + Sync> = authentication.try_into()?;
  info!("Logged in as {}", auth.auth_player_name());

  let jvm_args = format!(
    "-Xms512M -Xmx{}M -Dforgewrapper.librariesDir={} -Dforgewrapper.installer={} -Dforgewrapper.minecraft={} -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M",
    state.lock().unwrap().memory_max,
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
    .authentication(auth)
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
        println!("{}", &buf.trim_end());
        GAME_LOGS.lock().unwrap().push(buf.trim_end().to_string());
        buf.clear();
      }
    }

    if process.stderr().buffer().len() > 0 {
      if let Ok(length) = process.stderr().read_line(&mut buf) {
        if length > 0 {
          println!("{}", &buf.trim_end());
          GAME_LOGS.lock().unwrap().push(buf.trim_end().to_string());
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
fn get_launcher_logs_cache() -> Result<Vec<String>, TauriError> {
  Ok(LAUNCHER_LOGS_CACHE.lock().unwrap().clone().into())
}

#[tauri::command]
fn get_launcher_config(state: State<'_, Mutex<LauncherConfig>>) -> Result<LauncherConfig, TauriError> {
  Ok(state.lock().unwrap().clone())
}

#[tauri::command]
fn set_launcher_config(state: State<'_, Mutex<LauncherConfig>>, config: LauncherConfig) -> Result<(), TauriError> {
  let mut state = state.lock().unwrap();
  *state.deref_mut() = config;
  state.save_to_file()?;
  Ok(())
}

#[tauri::command]
fn login_offline(state: State<'_, Mutex<LauncherConfig>>, window: Window, username: String) -> Result<(), TauriError> {
  let mut state = state.lock().unwrap();
  let uuid = Uuid::new_v3(&Uuid::NAMESPACE_DNS, format!("OfflinePlayer:{username}").as_bytes()).to_string();
  state.authentication = Some(Authentication::Offline { username, uuid });
  state.broadcast_update(&window)?;
  state.save_to_file()?;
  Ok(())
}

#[tauri::command]
async fn login_msa(state: State<'_, Mutex<LauncherConfig>>, window: Window) -> Result<(), TauriError> {
  let ms_auth_token = msa_auth::get_msa_token(&window).await.map_err(|err| TauriError::Other(format!("Failed to get msa token: {}", err)))?;
  let auth = MsaMojangAuth::from(ms_auth_token).await.map_err(|err| TauriError::Other(format!("Failed to login: {}", err)))?;

  let mut state = state.lock().unwrap();
  state.authentication = Some(Authentication::Msa(auth));
  state.broadcast_update(&window)?;
  state.save_to_file()?;
  Ok(())
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
      LAUNCHER_LOGS.lock()?.push(msg.trim_end().to_string());
      if let Ok(mut cache) = LAUNCHER_LOGS_CACHE.lock() {
        cache.push_back(msg.trim_end().to_string());
        while cache.len() >= 1001 {
          let _ = cache.pop_front();
        }
      }
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
    .manage(Mutex::new(LauncherConfig::load_from_file().await))
    .invoke_handler(tauri::generate_handler![start_game, get_launcher_logs_cache, get_launcher_config, set_launcher_config, login_offline, login_msa])
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

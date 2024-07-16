use std::{ fs, io::BufRead, sync::Arc };

use log::{ debug, info, warn };
use minecraft_launcher_core::{
  bootstrap::{ auth::UserAuthentication, options::{ GameOptionsBuilder, LauncherOptions }, GameBootstrap },
  json::MCVersion,
  version_manager::{ downloader::progress::{ CallbackReporter, Event, ProgressReporter }, VersionManager },
};
use tauri::{ State, Window };

use crate::{
  app::{ error::TauriError, gui::GAME_STATUS_STATE },
  constants::{ LAUNCHER_DIRECTORY, LAUNCHER_NAME, LAUNCHER_VERSION },
  forge,
  game_status::GameStatus,
  java::{ check_java_dir, download_java },
  log_flusher::GAME_LOGS,
  modpack_downloader::ModpackInfo,
  DownloadProgress,
};

use super::{ error::StdError, state::LauncherState };

pub async fn real_start_game(state: State<'_, LauncherState>, window: Arc<Window>) -> Result<(), StdError> where Window: Sync {
  let authentication = {
    let config = state.launcher_config.lock().await;
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
  let mc_dir = &*LAUNCHER_DIRECTORY;
  let java_path = mc_dir.join("jre-runtime");
  let java_executable_path = java_path.join("bin").join("java.exe");

  GAME_STATUS_STATE.set(GameStatus::Downloading);
  debug!("Checking java runtime...");
  if !check_java_dir(&java_path) {
    info!("Java runtime not found. Downloading...");
    download_java(reporter.clone(), &java_path, "17").await.map_err(|err| TauriError::Other(format!("Failed to download java: {}", err)))?;
    info!("Java downloaded successfully!");
  }

  let mut downloader = state.modpack_downloader.lock().await;
  {
    debug!("Checking modpack...");
    let selected_options = state.launcher_config.lock().await.selected_options.clone();
    downloader.download_and_install(reporter.clone(), selected_options).await?;
  }

  let ModpackInfo { minecraft_version, forge_version, .. } = downloader.get_or_fetch_modpack_info().await?;
  let (forge_installer_path, forge_version_name) = forge::check_forge(mc_dir, minecraft_version, forge_version, &java_executable_path).await?;
  info!("Forge Version: {}", &forge_version_name);

  let auth: UserAuthentication = authentication.try_into()?;
  info!("Logged in as {}", auth.username);

  let jvm_args = format!(
    "-Xms512M -Xmx{}M -Dforgewrapper.librariesDir={} -Dforgewrapper.installer={} -Dforgewrapper.minecraft={} -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M",
    state.launcher_config.lock().await.memory_max,
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
    .game_dir(mc_dir.clone())
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

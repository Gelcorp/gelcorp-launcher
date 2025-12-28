use std::{ fs::{ self, create_dir_all }, path::PathBuf, process::Stdio, sync::{ Arc, Mutex } };

use futures::channel::oneshot;
use log::{ debug, error, info, warn };
use minecraft_launcher_core::{
  bootstrap::{ auth::UserAuthentication, options::{ GameOptionsBuilder, LauncherOptions }, process::GameProcessBuilder, GameBootstrap },
  java_manager::JavaRuntimeManager,
  json::MCVersion,
  version_manager::{ downloader::progress::{ CallbackReporter, Event, ProgressReporter }, VersionManager },
};
use tauri::Window;
use tokio::{ io::{ AsyncBufReadExt, AsyncRead, BufReader }, process::Command };

use crate::{
  app::{ error::LauncherError, game_status::GameStatus },
  constants::{ create_launcher_client, LAUNCHER_DIRECTORY, LAUNCHER_NAME, LAUNCHER_VERSION },
  forge,
  java::{ check_java_dir, download_java },
  log_flusher::GAME_LOGS,
  modpack_downloader::ModpackInfo,
  DownloadProgress,
};

use super::{ error::StdError, state::LauncherState };

pub async fn launch_game(state: &LauncherState, window: &Window) -> Result<(), StdError> where Window: Sync {
  let LauncherState { launcher_config, modpack_downloader, game_status } = state;
  let client = create_launcher_client(None);

  let authentication = {
    let config = launcher_config.lock().await;
    let authentication = config.authentication.as_ref();
    if authentication.is_none() {
      config.broadcast_update(window)?;
      return Err("Not logged in!".into());
    }
    authentication.unwrap().clone()
  };

  let reporter: ProgressReporter = {
    let window = window.clone();
    let progress = Mutex::new(None::<DownloadProgress>);
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
  let runtimes_dir = mc_dir.join("runtimes");
  create_dir_all(&runtimes_dir)?;

  game_status.set(GameStatus::Downloading);
  let runtime_manager = JavaRuntimeManager::load(&runtimes_dir, &client).await?;

  let mut downloader = modpack_downloader.lock().await;
  {
    debug!("Checking modpack...");
    let selected_options = launcher_config.lock().await.selected_options.clone();
    downloader.download_and_install(reporter.clone(), selected_options).await?;
  }

  let ModpackInfo { minecraft_version, forge_version, .. } = downloader.get_or_fetch_modpack_info().await?;

  let auth: UserAuthentication = authentication.try_into()?;
  info!("Logged in as {}", auth.username);

  let natives_dir = mc_dir.join("natives");
  if fs::remove_dir_all(&natives_dir).is_err() {
    warn!("Couldn't cleanup natives directory");
  }

  let mut game_opts = GameOptionsBuilder::default()
    .game_dir(mc_dir.clone())
    .java_path(PathBuf::new()) // Replaced later
    .launcher_options(LauncherOptions::new(LAUNCHER_NAME, LAUNCHER_VERSION))
    .authentication(auth)
    .natives_dir(natives_dir)
    .build()
    .map_err(|err| LauncherError::Other(format!("Failed to create game options: {err}")))?;
  let env_features = game_opts.env_features();

  reporter.setup("Fetching version manifest", Some(2));
  let mut version_manager = VersionManager::load(mc_dir, &env_features, Some(client)).await?;
  let manifest = version_manager.resolve_local_version(&MCVersion::new(minecraft_version), true, false).await?;
  reporter.status("Resolving local version");
  reporter.progress(1);
  info!("Queuing library & version downloads");
  if !manifest.applies_to_current_environment(&env_features) {
    return Err(format!("Version {} is is incompatible with the current environment", &minecraft_version).into());
  }
  reporter.done();

  debug!("Checking java runtime...");
  let objects_dir = mc_dir.join("assets").join("objects");
  if let Some(info) = &manifest.java_version {
    let java_component = &info.component;
    // TODO: also check platform
    if !runtime_manager.get_installed_runtimes()?.contains(java_component) {
      info!("Java runtime not found. Downloading...");
      runtime_manager.install_runtime(&objects_dir, java_component, &reporter).await?;
      info!("Java downloaded successfully!");
    }
    game_opts.java_path = runtime_manager.get_java_executable(java_component);
  } else {
    let runtime_dir = runtime_manager.get_runtime_dir("modpack-runtime");
    game_status.set(GameStatus::Downloading);
    debug!("Checking java runtime...");
    if !check_java_dir(&runtime_dir) {
      info!("Java runtime not found. Downloading...");
      download_java(reporter.clone(), &runtime_dir, "17").await.map_err(|err| LauncherError::Other(format!("Failed to download java: {}", err)))?;
      info!("Java downloaded successfully!");
    }
    game_opts.java_path = runtime_manager.get_java_executable("modpack-runtime");
  }

  let (forge_installer_path, forge_version_name) = forge::check_forge(
    mc_dir,
    &minecraft_version.to_string(),
    forge_version,
    &game_opts.java_path
  ).await?;
  info!("Forge Version: {}", &forge_version_name);
  let mc_version = MCVersion::new(&forge_version_name);

  let guard = launcher_config.lock().await;
  let jvm_args = format!(
    "-Xms{}M -Xmx{}M -Dforgewrapper.librariesDir={} -Dforgewrapper.installer={} -Dforgewrapper.minecraft={} {}",
    guard.memory_max / 2,
    guard.memory_max,
    mc_dir.join("libraries").display(),
    forge_installer_path.display(),
    mc_dir.join(format!("versions/{0}/{0}.jar", &forge_version_name)).display(),
    guard.jre_flags
  );
  drop(guard);
  game_opts.jvm_args.replace(jvm_args.split(' ').map(String::from).collect());

  version_manager.refresh().await?;
  let manifest = version_manager.resolve_local_version(&mc_version, true, false).await?;
  if !manifest.applies_to_current_environment(&env_features) {
    return Err(format!("Version {} is is incompatible with the current environment", &mc_version).into());
  }
  reporter.done();
  version_manager.download_required_files(&manifest, &reporter, None, None).await?;

  let GameProcessBuilder { arguments, java_path, directory } = GameBootstrap::new(game_opts)
    .prepare_launch(&manifest)
    .map_err(|err| LauncherError::Other(format!("Failed to launch the game: {err}")))?;

  game_status.set(GameStatus::Playing);

  let mut process = Command::new(java_path.unwrap())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .current_dir(directory.unwrap())
    .args(arguments)
    .spawn()
    .map_err(|err| LauncherError::Other(format!("Failed to launch the game: {err}")))?;
  let stdout = BufReader::new(process.stdout.take().unwrap());
  let stderr = BufReader::new(process.stderr.take().unwrap());

  fn log_lines(reader: BufReader<impl AsyncRead + Unpin + Send + 'static>) -> oneshot::Sender<()> {
    let (tx, mut rx) = oneshot::channel();

    tokio::spawn(async move {
      let mut lines = reader.lines();

      while let Ok(None) = rx.try_recv() {
        match lines.next_line().await {
          // TODO: find out why this happens
          Ok(Some(line)) if line != "false" => {
            let line = line.trim_end();
            println!("{}", &line);
            GAME_LOGS.log(line);
          }
          Err(err) => error!("Failed to read game output: {}", err),
          _ => (),
        }
      }
    });
    tx
  }

  let cancel_stdout = log_lines(stdout);
  let cancel_stderr = log_lines(stderr);

  let exit_status = process.wait().await;

  let _ = cancel_stdout.send(());
  let _ = cancel_stderr.send(());

  let code = exit_status?.code().unwrap_or(-1);
  if code == 0 {
    info!("Game exited successfully");
    Ok(())
  } else {
    info!("Game exited with code {code}");
    Err(format!("Failed to launch the game. Process exited with code {code}").into())
  }
}

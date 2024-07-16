use tokio::sync::Mutex;

use crate::{ config::LauncherConfig, modpack_downloader::ModpackDownloader };

pub struct LauncherState {
  pub launcher_config: Mutex<LauncherConfig>,
  pub modpack_downloader: Mutex<ModpackDownloader>,
}

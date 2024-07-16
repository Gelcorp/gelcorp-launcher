use tokio::sync::Mutex;

use crate::{ config::LauncherConfig, modpack_downloader::ModpackDownloader };

use super::game_status::GameStatusState;

pub struct LauncherState {
  pub launcher_config: Mutex<LauncherConfig>,
  pub modpack_downloader: Mutex<ModpackDownloader>,
  pub game_status: GameStatusState,
}

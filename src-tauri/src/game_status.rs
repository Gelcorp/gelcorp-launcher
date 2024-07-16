use std::sync::Mutex;

use serde::Serialize;
use tauri::Window;

#[derive(Serialize, Clone, Default)]
pub enum GameStatus {
  #[default] Idle,
  Downloading,
  Playing,
}

#[derive(Default)]
pub struct GameStatusState {
  status: Mutex<GameStatus>,
  window: Mutex<Option<Window>>,
}

impl GameStatusState {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn set_window(&self, window: Window) {
    self.window.lock().unwrap().replace(window);
  }

  pub fn set(&self, status: GameStatus) {
    *self.status.lock().unwrap() = status.clone();
    if let Some(window) = &*self.window.lock().unwrap() {
      let _ = window.emit("game_status", status);
    }
  }

  pub fn get(&self) -> GameStatus {
    self.status.lock().unwrap().clone()
  }
}

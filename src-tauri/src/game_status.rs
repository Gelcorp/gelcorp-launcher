use std::sync::Mutex;

use serde::Serialize;
use tauri::Window;

#[derive(Serialize, Clone)]
pub enum GameStatus {
  Idle,
  Downloading,
  Playing,
}

pub struct GameStatusState {
  pub status: Mutex<GameStatus>,
  pub window: Mutex<Option<Window>>,
}

impl GameStatusState {
  pub fn new() -> Self {
    Self { status: Mutex::new(GameStatus::Idle), window: Mutex::new(None) }
  }

  pub fn set_window(&self, window: Window) {
    *self.window.lock().unwrap() = Some(window);
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

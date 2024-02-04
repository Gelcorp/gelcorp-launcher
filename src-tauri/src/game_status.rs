use std::sync::Mutex;

use serde::Serialize;
use tauri::Window;

#[derive(Serialize, Clone)]
pub enum GameStatus {
  Idle,
  Downloading,
  Playing,
}

pub struct GameStatusState(Mutex<GameStatus>, Mutex<Option<Window>>);

impl GameStatusState {
  pub fn new() -> Self {
    Self(Mutex::new(GameStatus::Idle), Mutex::new(None))
  }

  pub fn set_window(&self, window: Window) {
    self.1.lock().unwrap().replace(window);
  }

  pub fn set(&self, status: GameStatus) {
    *self.0.lock().unwrap() = status.clone();
    if let Some(window) = &*self.1.lock().unwrap() {
      let _ = window.emit("game_status", status);
    }
  }

  pub fn get(&self) -> GameStatus {
    self.0.lock().unwrap().clone()
  }
}

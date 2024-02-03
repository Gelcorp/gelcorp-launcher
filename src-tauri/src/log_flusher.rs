use std::{ collections::VecDeque, mem, sync::Mutex, thread::{ self, sleep }, time::Duration };

use tauri::{ plugin::{ Builder, TauriPlugin }, AppHandle, Manager, Runtime };

pub static GAME_LOGS: LogFlusher = LogFlusher::new("game_logs");
pub static LAUNCHER_LOGS: LogFlusher = LogFlusher::new("launcher_logs");
static ALL_LOGS: [&LogFlusher; 2] = [&GAME_LOGS, &LAUNCHER_LOGS];

pub struct LogFlusher {
  id: &'static str,
  buffer: Mutex<Vec<String>>,
  logs: Mutex<VecDeque<String>>,
}

impl LogFlusher {
  pub const fn new(id: &'static str) -> Self {
    Self {
      id,
      buffer: Mutex::new(vec![]),
      logs: Mutex::new(VecDeque::new()),
    }
  }

  pub fn get_all(&self) -> Vec<String> {
    self.logs.lock().unwrap().iter().cloned().collect()
  }

  // TODO: avoid cloning
  pub fn log(&self, text: impl AsRef<str>) {
    let text = text.as_ref().to_string();
    self.buffer.lock().unwrap().push(text.clone());

    let mut logs = self.logs.lock().unwrap();
    logs.push_back(text);
    while logs.len() >= 1001 {
      let _ = logs.pop_front();
    }
  }

  pub fn flush<R: Runtime>(&self, app: &AppHandle<R>) {
    let mut logs = self.buffer.lock().unwrap();
    if !logs.is_empty() {
      app.emit_all(self.id, mem::take(&mut *logs)).unwrap();
    }
  }
}

//

pub fn flush_all_logs<R: Runtime>(app: &AppHandle<R>) {
  for flusher in &ALL_LOGS {
    flusher.flush(app);
  }
}

fn setup_log_flusher<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
  let app = app.clone();
  let builder = thread::Builder::new();
  builder
    .name("launcher-log-watcher".into())
    .spawn(move || {
      loop {
        flush_all_logs(&app);
        sleep(Duration::from_millis(5));
      }
    })
    .expect("Failed to spawn log watcher thread");
  Ok(())
}

#[tauri::command]
fn get_logs(id: &str) -> Result<Vec<String>, String> {
  for flusher in &ALL_LOGS {
    if flusher.id == id {
      return Ok(flusher.get_all());
    }
  }
  Err(format!("Unknown log id: {id}"))
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("log-flusher").setup(setup_log_flusher).invoke_handler(tauri::generate_handler![get_logs]).build()
}

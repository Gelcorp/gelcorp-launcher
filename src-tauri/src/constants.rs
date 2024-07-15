use std::{ env::var, path::{ Path, PathBuf } };

use once_cell::sync::Lazy;
use regex::{ Captures, Regex };

pub const LAUNCHER_NAME: &str = env!("LAUNCHER_NAME");
pub const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub static LAUNCHER_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| resolve_path(env!("GAME_DIR_PATH")));

fn resolve_path(path: impl AsRef<Path>) -> PathBuf {
  let path = path.as_ref();
  let regex = Regex::new(r"%([a-zA-Z0-9]+)%").unwrap();
  let mut new_path_buf = PathBuf::new();

  for component in path.components() {
    if let Some(component_str) = component.as_os_str().to_str() {
      let replaced_component = regex
        .replace_all(component_str, |captures: &Captures| {
          match var(&captures[1]) {
            Ok(value) => value,
            Err(_) => captures[0].to_string(),
          }
        })
        .to_string();

      new_path_buf.push(Path::new(&replaced_component));
    } else {
      new_path_buf.push(component);
    }
  }

  new_path_buf
}

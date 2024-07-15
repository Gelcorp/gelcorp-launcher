use std::path::PathBuf;

use expand_str::expand_string_with_env;
use once_cell::sync::Lazy;

pub const LAUNCHER_NAME: &str = env!("LAUNCHER_NAME");
pub const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub static LAUNCHER_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| expand_string_with_env(env!("GAME_DIR_PATH")).unwrap().into());

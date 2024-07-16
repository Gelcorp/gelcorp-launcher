use std::path::PathBuf;

use expand_str::expand_string_with_env;
use once_cell::sync::Lazy;

pub const LAUNCHER_NAME: &str = env!("LAUNCHER_NAME");
pub const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const UPDATE_ENDPOINTS: &str = env!("UPDATE_ENDPOINTS");
pub const MSA_CLIENT_ID: &str = env!("MSA_CLIENT_ID");

// OAuth Constants
pub const REDIRECT_URL: &str = "https://login.live.com/oauth20_desktop.srf";
pub const AUTHORIZE_URL: &str = "https://login.live.com/oauth20_authorize.srf";
pub const TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";

pub static LAUNCHER_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| expand_string_with_env(env!("GAME_DIR_PATH")).unwrap().into());

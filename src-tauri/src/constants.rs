use std::path::PathBuf;

use expand_str::expand_string_with_env;
use minecraft_launcher_core::bootstrap::options::ProxyOptions;
use once_cell::sync::Lazy;
use reqwest::Client;

pub const LAUNCHER_NAME: &str = env!("LAUNCHER_NAME");
pub const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const UPDATE_ENDPOINTS: &str = env!("UPDATE_ENDPOINTS");
pub const MSA_CLIENT_ID: &str = env!("MSA_CLIENT_ID");

// OAuth Constants
pub const REDIRECT_URL: &str = "https://login.live.com/oauth20_desktop.srf";
pub const AUTHORIZE_URL: &str = "https://login.live.com/oauth20_authorize.srf";
pub const TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";

pub const LAUNCHER_USER_AGENT: &str = concat!(env!("LAUNCHER_NAME"), '/', env!("CARGO_PKG_VERSION"));
pub static LAUNCHER_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| expand_string_with_env(env!("GAME_DIR_PATH")).unwrap().into());

pub fn create_launcher_client(proxy: Option<ProxyOptions>) -> Client {
  let proxy = proxy.as_ref().and_then(ProxyOptions::create_http_proxy);
  let mut builder = Client::builder().user_agent(LAUNCHER_USER_AGENT);
  if let Some(proxy) = proxy {
    builder = builder.proxy(proxy);
  }
  builder.build().unwrap_or_default()
}

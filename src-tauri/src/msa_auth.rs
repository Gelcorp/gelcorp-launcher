use std::sync::{ Arc, Mutex };

use chrono::Utc;
use oauth2::{
  basic::{ BasicClient, BasicTokenType },
  ClientId,
  AuthUrl,
  TokenUrl,
  AuthType,
  RedirectUrl,
  PkceCodeChallenge,
  CsrfToken,
  Scope,
  AuthorizationCode,
  reqwest::async_http_client,
  TokenResponse,
  StandardTokenResponse,
  EmptyExtraTokenFields,
};
use reqwest::Url;
use serde::{ Deserialize, Serialize };
use tauri::{ WindowBuilder, WindowUrl, Manager, Window };
use thiserror::Error;

const CLIENT_ID: &str = std::env!("MSA_CLIENT_ID");
const REDIRECT_URL: &str = "https://login.live.com/oauth20_desktop.srf";
const AUTHORIZE_URL: &str = "https://login.live.com/oauth20_authorize.srf";
const TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";

pub async fn get_msa_token(owner_window: &Window) -> Result<MSAuthToken, Box<dyn std::error::Error>> {
  // Generate auth link and pkce challenge
  let client = BasicClient::new(
    ClientId::new(CLIENT_ID.to_string()),
    None,
    AuthUrl::new(AUTHORIZE_URL.to_string())?,
    Some(TokenUrl::new(TOKEN_URL.to_string())?)
  )
    .set_auth_type(AuthType::RequestBody)
    .set_redirect_uri(RedirectUrl::new(REDIRECT_URL.to_string())?);

  let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

  let (auth_link, csrf_state) = client
    .authorize_url(CsrfToken::new_random)
    .add_scope(Scope::new("XboxLive.signin offline_access".to_string()))
    .set_pkce_challenge(pkce_code_challenge)
    .add_extra_param("prompt", "select_account")
    .url();

  // Open window and wait for redirect
  let window = WindowBuilder::new(&owner_window.app_handle(), "msa_auth", WindowUrl::External(auth_link))
    .title("Login with Microsoft")
    .maximizable(false)
    .resizable(false)
    .max_inner_size(500_f64, 650_f64)
    .focused(true)
    .owner_window(owner_window.hwnd().unwrap())
    .build()?;

  let is_window_closed = Arc::new(Mutex::new(false));
  {
    let is_window_closed = Arc::clone(&is_window_closed);
    window.on_window_event(move |event| {
      if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        let mut is_window_closed = is_window_closed.lock().unwrap();
        *is_window_closed = true;
        api.prevent_close();
      }
    });
  }

  let (code, state) = (loop {
    if is_window_closed.lock().is_ok_and(|closed| *closed) {
      let _ = window.close();
      break Err(MSAuthError::LoginCancelled);
    }

    let url = window.url();
    if !url.as_str().to_string().starts_with(REDIRECT_URL) {
      continue;
    }
    let mut params = url.query_pairs();
    let code = params.find(|(key, _)| key == "code").unwrap().1;
    let state = params.find(|(key, _)| key == "state").unwrap().1;
    break Ok((AuthorizationCode::new(code.to_string()), CsrfToken::new(state.to_string())));
  })?;
  window.close()?;

  // Check CSRF challenge

  if state.secret() != csrf_state.secret() {
    return Err(MSAuthError::CsrfMismatch(state.secret().clone(), csrf_state.secret().clone()))?;
  }

  let tokens = client.exchange_code(code).set_pkce_verifier(pkce_code_verifier).request_async(async_http_client).await?;

  let tokens: MSAuthToken = MSATokenResponse::from(tokens).into();
  Ok(tokens)
}

#[derive(Debug, Error)]
pub enum MSAuthError {
  #[error("Login process was cancelled by the user")] LoginCancelled,
  #[error("CSRF state mismatch ({0} != {1})")] CsrfMismatch(String, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MSAuthToken {
  pub access_token: String,
  pub refresh_token: String,
  pub expiration_date: i64,
}

impl MSAuthToken {
  pub fn validate(&self) -> bool {
    Utc::now().timestamp_millis() < self.expiration_date
  }

  pub async fn refresh(&mut self, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    if self.validate() && !force {
      return Ok(());
    }
    let url = Url::parse_with_params(
      TOKEN_URL,
      &[
        ("client_id", CLIENT_ID),
        ("refresh_token", &self.refresh_token),
        ("grant_type", "refresh_token"),
      ]
    ).unwrap();

    let MSAuthToken { access_token, refresh_token, expiration_date } = reqwest::Client
      ::new()
      .post(url.as_str())
      .body(url.query().unwrap().to_string())
      .header("Content-Type", "application/x-www-form-urlencoded")
      .send().await?
      .error_for_status()?
      .json::<MSATokenResponse>().await?
      .into();

    self.access_token = access_token;
    self.refresh_token = refresh_token;
    self.expiration_date = expiration_date;
    Ok(())
  }
}

#[derive(Debug, Serialize, Deserialize)]
struct MSATokenResponse {
  access_token: String,
  refresh_token: String,
  #[serde(rename = "expires_in")]
  expires_in_seconds: u64, // seconds
}

impl From<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> for MSATokenResponse {
  fn from(tokens: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>) -> Self {
    MSATokenResponse {
      access_token: tokens.access_token().secret().to_string(),
      refresh_token: tokens.refresh_token().unwrap().secret().to_string(),
      expires_in_seconds: tokens.expires_in().unwrap().as_secs(),
    }
  }
}

impl Into<MSAuthToken> for MSATokenResponse {
  fn into(self) -> MSAuthToken {
    MSAuthToken {
      access_token: self.access_token,
      refresh_token: self.refresh_token,
      expiration_date: Utc::now().timestamp() + ((self.expires_in_seconds * 1000u64) as i64),
    }
  }
}

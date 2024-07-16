use serde::Serialize;
use thiserror::Error;

pub type StdError = Box<dyn std::error::Error>;

#[derive(Debug, Error)]
pub enum LauncherError {
  #[error(transparent)] Reqwest(#[from] reqwest::Error),
  #[error(transparent)] Io(#[from] std::io::Error),
  #[error("{0}")] Other(String),
}

impl From<StdError> for LauncherError {
  fn from(error: StdError) -> Self {
    Self::Other(error.to_string())
  }
}

impl Serialize for LauncherError {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
    serializer.serialize_str(&self.to_string())
  }
}

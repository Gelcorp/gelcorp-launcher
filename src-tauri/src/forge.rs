use std::{ fs::{ self, create_dir_all, File }, path::PathBuf };

use forge_downloader::{
  download_utils::forge::ForgeVersionHandler,
  forge_client_install::ForgeClientInstall,
  forge_installer_profile::ForgeVersionInfo,
};
use log::{ debug, info };
use regex::Regex;
use reqwest::Client;
use serde_json::json;

use crate::{ StdError, TauriError };

pub async fn check_forge(mc_dir: &PathBuf, mc_version: &str, forge_version: &str, java_path: &PathBuf) -> Result<(PathBuf, String), TauriError> {
  let versions_dir = mc_dir.join("versions");

  // Fetch version
  let version_handler = ForgeVersionHandler::new().await?;
  let version_info = version_handler.get_by_forge_version(forge_version).expect("Failed to get forge version");
  let installer_path = version_info.get_artifact().get_local_path(&mc_dir.join("libraries"));

  if versions_dir.is_dir() && installer_path.is_file() {
    let forge_folder_re = Regex::new(&format!("{}.+{}", mc_version, forge_version)).unwrap();
    let forge_version_name = versions_dir
      .read_dir()?
      .into_iter()
      .filter_map(|dir| dir.ok())
      .filter_map(|dir| dir.file_name().into_string().ok())
      .find(|name| forge_folder_re.is_match(name));
    if let Some(name) = forge_version_name {
      return Ok((installer_path, name));
    }
  }

  // Download installer if needed
  if !installer_path.is_file() {
    if let Some(parent) = installer_path.parent() {
      create_dir_all(parent)?;
    }

    let bytes = Client::new().get(&version_info.get_installer_url()).send().await?.error_for_status()?.bytes().await?;
    fs::write(&installer_path, &bytes)?;
  }

  // Open installer
  let mut install_handler = ForgeClientInstall::new(installer_path.clone(), java_path.clone())?;
  let forge_version_id = install_handler.get_profile().get_version_id();

  let forge_version_path = mc_dir.join(format!("versions/{id}/{id}.json", id = &forge_version_id));
  if !forge_version_path.is_file() {
    info!("Forge not installed! Setting up forge...");
    install_handler.install_forge(&mc_dir, |_| true).await?;
    debug!("Setting up forge wrapper...");
    setup_forge_wrapper(&forge_version_path)?;
    info!("Forge installed!");
  }
  Ok((installer_path, forge_version_id))
}

fn setup_forge_wrapper(forge_version_path: &PathBuf) -> Result<(), StdError> {
  let mut version_info: ForgeVersionInfo = serde_json::from_reader(File::open(forge_version_path)?)?;
  let wrapper_lib =
    json!({
    "downloads": {
      "artifact": {
        "sha1": "155ac9f4e5f65288eaacae19025ac4d9da1f0ef2",
        "size": 34910,
        "url": "https://github.com/ZekerZhayard/ForgeWrapper/releases/download/1.5.7/ForgeWrapper-1.5.7.jar"
      }
    },
    "name": "io.github.zekerzhayard:ForgeWrapper:1.5.7"
  });
  version_info.libraries.push(serde_json::from_value(wrapper_lib)?);
  version_info.main_class = "io.github.zekerzhayard.forgewrapper.installer.Main".to_string();
  serde_json::to_writer_pretty(&mut File::create(forge_version_path)?, &version_info)?;
  Ok(())
}

pub mod keys;

use std::{ fs::{ self, create_dir_all }, io::{ Cursor, Write }, path::PathBuf, sync::Arc, time::Duration };

use aes::{ cipher::{ block_padding::Pkcs7, BlockDecryptMut, KeyIvInit }, Aes256 };
use cbc::Decryptor;
use chrono::Utc;
use futures::StreamExt;
use gelcorp_modpack::{ reader::zip::ModpackArchiveReader, types::ModOptional };
use log::{ debug, error, info, warn };
use minecraft_launcher_core::progress_reporter::ProgressReporter;
use reqwest::{ Url, Client, ClientBuilder };
use rsa::{ sha2::Sha256, Pkcs1v15Sign };
use serde::{ Deserialize, Serialize };
use sha1::Digest;
use tokio::{ fs::File, io::{ AsyncWriteExt, BufWriter, self } };
use zip::ZipArchive;

use crate::modpack_downloader::keys::{ get_aes_keys, get_public_key };

/*
Process:
  0.  Fetch modpack info to see what parts the launcher has to download
  0.5 Reconstruct Encrypted bundle by joining the parts
  1.  Download modpack (if remote sha256 is different from locally calculated sha256, or if the modpack wasn't downloaded yet)
        - Sha256 should be calculated from encrypted version
  2.  Verify modpack signature with public key
  2.5 Save modpack for future checks
  3.  Decypher modpack
  4.  Parse modpack and extract files
  5.  Done!
  
Modpack structure:
mods/                     // Essential mods, mandatory
  - libs/                 // Essential libs (for essential mods, mandatory) 
  - {optional_mods}/      // Optional mods (performance, visuals, etc)
    - libs/
    - {mod}.jar
  - {mod}.jar             
.minecraft/               // Config Files, files to extract in general (check config)
manifest.json
  - format_version: 1    // Format version of deserializer
*/

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModpackInfo {
  pub parts: Vec<String>,
  pub minecraft_version: String,
  pub forge_version: String,

  #[serde(default)]
  pub optionals: Vec<ModOptional>,
  pub checksum: String, // Sha256  (hex)
  pub signature: String, // Rsa     (hex)
}

#[derive(Debug, Clone)]
pub struct ModpackProvider {
  base_url: Url,
  client: Client,
}

impl ModpackProvider {
  pub fn new(base_url: &str) -> Self {
    let base_url = if !base_url.ends_with("/") { Url::parse(&(base_url.to_owned() + "/")).unwrap() } else { Url::parse(base_url).unwrap() };
    Self {
      base_url,
      client: ClientBuilder::new().connect_timeout(Duration::from_millis(5000)).build().unwrap(),
    }
  }

  pub async fn fetch_info(&self) -> Result<ModpackInfo, Box<dyn std::error::Error>> {
    let url = self.base_url.join("modpack_info.json")?;
    let modpack_info: ModpackInfo = self.client.get(url).send().await?.error_for_status()?.json().await?;
    Ok(modpack_info)
  }

  pub async fn reconstruct_encrypted_modpack(&self, monitor: Arc<ProgressReporter>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let ModpackInfo { parts, .. } = self.fetch_info().await?;
    assert!(!parts.is_empty(), "No modpack parts provided");
    let tmp_dir = std::env::temp_dir().join(format!("modpack-{}", Utc::now().timestamp_millis()));
    create_dir_all(&tmp_dir)?;

    async fn download_part(client: &Client, url: &Url, target: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
      let response = client.get(url.as_str()).send().await?.error_for_status()?;
      let mut stream = response.bytes_stream();
      let mut file = File::create(target).await?;
      while let Some(Ok(chunk)) = stream.next().await {
        file.write_all(&chunk).await?;
        file.flush().await?;
      }
      Ok(())
    }

    // TODO: concurrency!!!
    monitor.set("Downloading modpack parts", 0, parts.len() as u32);
    let mut progress = 0;
    for file_name in &parts {
      let mut attempts = 0;
      let url = self.base_url.join(file_name)?;
      let target = tmp_dir.join(file_name);
      while attempts < 5 {
        info!("Downloading {} (attempt {})", file_name, attempts + 1);
        let result = download_part(&self.client, &url, &target).await;
        if let Err(e) = result {
          error!("Error downloading {}: {}", file_name, e);
          attempts += 1;
          continue;
        }
        progress += 1;
        monitor.set_progress(progress);
        break;
      }
    }

    // Join parts
    monitor.set_progress(0);
    monitor.set_status("Joining modpack parts");
    let mut buf = vec![];
    for (i, file_name) in parts.iter().enumerate() {
      let file = File::open(tmp_dir.join(&file_name)).await?;
      let mut reader = BufWriter::new(file);
      io::copy(&mut reader, &mut buf).await?;
      monitor.set_progress((i + 1) as u32);
    }

    Ok(buf)
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalInstallInfo {
  checksum: String,
  optionals: Vec<String>,
  extracted_mods: Vec<String>,
}

type Aes256CbcDec = Decryptor<Aes256>;
type StdError = Box<dyn std::error::Error>;

pub struct ModpackDownloader {
  providers: Vec<ModpackProvider>,
  mc_dir: PathBuf,
  modpack_info: Option<ModpackInfo>,
}

impl ModpackDownloader {
  pub fn new(mc_dir: PathBuf, providers: Vec<ModpackProvider>) -> Self {
    Self { providers, mc_dir, modpack_info: None }
  }

  pub async fn get_or_fetch_modpack_info(&mut self) -> Result<&ModpackInfo, StdError> {
    if self.modpack_info.is_none() {
      self.modpack_info.replace(self.fetch_latest_modpack_info().await?);
    }
    Ok(self.modpack_info.as_ref().unwrap())
  }

  pub async fn download_and_install(&mut self, monitor: Arc<ProgressReporter>, chosen_optionals: Vec<String>) -> Result<(), StdError> {
    let (public_key, aes_keys) = (get_public_key()?, get_aes_keys()?);

    let local_modpack_dir_path = &self.mc_dir.join("modpack");
    // TODO: let local_install_info_path = local_modpack_dir_path.join("install_info.json");
    let local_modpack_path = local_modpack_dir_path.join("modpack.enc.zip");
    let local_modpack_sig_path = local_modpack_dir_path.join("modpack.enc.sig");

    if local_modpack_path.is_file() && local_modpack_sig_path.is_file() {
      monitor.set("Verifying local modpack", 0, 1);
      info!("Local modpack found! Verifying...");
      let signature = fs::read(&local_modpack_sig_path)?;
      let local_modpack_sha256 = get_local_modpack_enc_hash(&local_modpack_path)?;
      if let Err(_) = public_key.verify(Pkcs1v15Sign::new::<Sha256>(), &local_modpack_sha256, &signature) {
        warn!("Invalid local modpack! Downloading it again...");
        fs::remove_file(&local_modpack_path)?;
        fs::remove_file(&local_modpack_sig_path)?;
      }
      monitor.set_progress(1);
    }

    let local_modpack_sha256 = get_local_modpack_enc_hash(&local_modpack_path).ok();

    let mut provider_success = false;
    let mut should_install = false;
    info!("Checking for updates...");
    let total_providers = self.providers.len() as u32;
    for (i, provider) in self.providers.iter().enumerate() {
      monitor.set(format!("Trying modpack provider {} ({}/{})", provider.base_url.as_str(), i + 1, total_providers), i as u32, total_providers);
      info!(" - Trying provider '{}'", provider.base_url);
      let remote_info = provider.fetch_info().await;
      if let Err(err) = remote_info {
        warn!("   Error checking provider: {}", err);
        continue;
      }
      let remote_info = remote_info?;
      provider_success = true;

      let remote_checksum = hex::decode(&remote_info.checksum);
      if let Err(err) = remote_checksum {
        warn!("   Error decoding remote checksum: {}", err);
        continue;
      }
      let remote_checksum: [u8; 32] = remote_checksum?.try_into().map_err(|_| format!("Checksum is not 32 bytes: {}", remote_info.checksum))?;

      if let Some(local_checksum) = local_modpack_sha256 {
        info!("   Found local modpack info! Checking for updates...");
        if local_checksum == remote_checksum {
          info!("   Local modpack is up to date!");
          break;
        }
        info!("   Local modpack is outdated! Updating...");
      } else {
        // No previous modpack downloaded
        info!("   Modpack not found. Downloading...");
      }

      // Download it
      info!("   Downloading modpack...");
      let remote_modpack = provider.reconstruct_encrypted_modpack(monitor.clone()).await?;
      monitor.clear();

      // Verify installation
      info!("   Remote modpack downloaded! Verifying checksum...");
      let manual_checksum: [u8; 32] = <Sha256 as Digest>::digest(&remote_modpack).into();
      if manual_checksum != remote_checksum {
        warn!("   Checksum mismatch. Download failed! (Remote: {}, Downloaded: {})", hex::encode(&remote_checksum), hex::encode(&manual_checksum));
        continue;
      }

      let signature = hex::decode(&remote_info.signature)?;
      if let Err(err) = public_key.verify(Pkcs1v15Sign::new::<Sha256>(), &manual_checksum, &signature) {
        warn!("   Failed to check signature. Download failed! ({})", err);
      }

      info!("   Modpack verified! Saving files...");
      create_dir_all(&local_modpack_dir_path)?;
      fs::File::create(&local_modpack_path)?.write_all(&remote_modpack)?;
      fs::File::create(&local_modpack_sig_path)?.write_all(&signature)?;
      should_install = true;
      info!("Modpack download completed!");
      self.modpack_info = Some(remote_info);
      break;
    }
    monitor.clear();
    if !local_modpack_path.is_file() || !provider_success {
      if !provider_success {
        error!("All providers failed! Quitting...");
      }
      error!("Failed to download modpack!");
      return Err("Failed to download modpack".into());
    }

    // TODO: Remove this, check if optionals change, or execute each time (because of replace = true config files ???)
    debug!("Should the modpack be installed? {}", should_install); // TODO: remove, just to stop the warning
    should_install = true;

    if !should_install {
      return Ok(());
    }

    let aes_decoder = Aes256CbcDec::new_from_slices(aes_keys.key(), aes_keys.iv())?;
    monitor.set("Installing modpack", 0, 1);
    if let Err(err) = self.try_install_modpack(aes_decoder, &local_modpack_path, chosen_optionals) {
      error!("Failed to install modpack: {}", err);
      fs::remove_file(&local_modpack_path)?;
      fs::remove_file(&local_modpack_sig_path)?;
      monitor.clear();
      return Err(format!("Failed to install modpack: {err}").into());
    }
    monitor.set_progress(1);
    Ok(())
  }

  fn try_install_modpack(&self, aes_decoder: Decryptor<Aes256>, local_modpack_path: &PathBuf, chosen_optionals: Vec<String>) -> Result<(), StdError> {
    info!("Decoding modpack...");
    let mut bytes = fs::read(&local_modpack_path)?;
    let decoded = aes_decoder.decrypt_padded_mut::<Pkcs7>(&mut bytes).map_err(|err| format!("Failed to decrypt modpack: {err}"))?;
    let archive = ZipArchive::new(Cursor::new(decoded)).map_err(|err| format!("Failed to open modpack archive: {err}"))?;

    info!("Installing modpack...");
    let mut modpack = ModpackArchiveReader::try_from(archive)?;
    modpack.install(&self.mc_dir, chosen_optionals)?;
    info!("Modpack installed!");
    Ok(())
  }

  async fn fetch_latest_modpack_info(&self) -> Result<ModpackInfo, StdError> {
    for provider in &self.providers {
      if let Ok(modpack_info) = provider.fetch_info().await {
        return Ok(modpack_info);
      }
    }
    Err("No modpack info found".into())
  }
}

fn get_local_modpack_enc_hash(local_modpack_path: &PathBuf) -> Result<[u8; 32], StdError> {
  let bytes = fs::read(&local_modpack_path)?;
  let sum = <Sha256 as Digest>::digest(bytes);
  Ok(sum.into())
}

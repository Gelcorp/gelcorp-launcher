use std::{
  path::PathBuf,
  time::Duration,
  env::consts::{ OS, ARCH },
  fs::{ create_dir_all, File, self },
  io::{ BufWriter, Write, self },
  process::{ Command, Stdio },
  os::windows::process::CommandExt,
  sync::Arc,
};

use futures::StreamExt;
use minecraft_launcher_core::progress_reporter::ProgressReporter;
use reqwest::ClientBuilder;
use zip::ZipArchive;

pub fn check_java_dir(java_dir: &PathBuf) -> bool {
  let java = java_dir.join("bin").join("java.exe");
  if !java.is_file() {
    return false;
  }
  Command::new(java)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .creation_flags(0x08000000)
    .arg("--version")
    .status()
    .is_ok_and(|c| c.success())
}

pub async fn download_java(reporter: Arc<ProgressReporter>, java_dir: &PathBuf, java_version: &str) -> Result<(), Box<dyn std::error::Error>> {
  let client = ClientBuilder::new().connect_timeout(Duration::from_secs(30)).build()?;
  let os = match OS {
    "macos" => "mac",
    os => os,
  };
  let arch = match ARCH {
    "x86" => "x86",
    "x86_64" => "x64",
    arch => arch,
  };
  let url = format!("https://api.adoptium.net/v3/binary/latest/{java_version}/ga/{os}/{arch}/jre/hotspot/normal/eclipse");
  let response = client.get(url).send().await?.error_for_status()?;

  let temp_file_path = java_dir.join("runtime.tmp");
  create_dir_all(java_dir)?;
  // Stream download into temp file (avoiding unnecesary memory usage)
  {
    // Downloading java
    let mut file = File::create(&temp_file_path)?;
    let status = format!("Downloading java {}", java_version);
    let mut current = 0;
    let total = response.content_length().unwrap() as u32;

    reporter.set(&status, current, total);

    let mut writer = BufWriter::new(&mut file);
    let mut stream = response.bytes_stream();
    while let Some(Ok(chunk)) = stream.next().await {
      current += chunk.len() as u32;
      writer.write_all(&chunk)?;
      reporter.set_progress(current);
    }

    reporter.clear();
  }
  // Extract file
  {
    let file = File::open(&temp_file_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut progress = 0;
    let total = archive.len();

    for i in 0..total {
      let mut zip_archive = archive.by_index(i)?;
      if let Some((_, file_name)) = zip_archive.name().split_once("/") {
        progress += 1;
        reporter.set(&format!("Extracting {}", file_name), progress, total as u32);
        let target_path = java_dir.join(file_name);
        if file_name.is_empty() {
          continue;
        }
        if target_path.exists() {
          continue;
        }

        if file_name.ends_with("/") {
          create_dir_all(target_path)?;
        } else {
          create_dir_all(target_path.parent().unwrap())?;
          let mut file = File::create(target_path)?;
          io::copy(&mut zip_archive, &mut file)?;
        }
      }
    }
  }
  reporter.clear();
  let _ = fs::remove_file(temp_file_path);

  Ok(())
}

#[cfg(test)]
mod tests {
  use std::{ env::temp_dir, sync::Arc };
  use super::*;

  #[tokio::test]
  async fn test_download() -> Result<(), Box<dyn std::error::Error>> {
    let java_dir = temp_dir().join("java-download-test");
    if check_java_dir(&java_dir) {
      println!("Java already exists");
      return Ok(());
    }
    download_java(Arc::new(ProgressReporter::default()), &java_dir, "17").await?;
    assert!(check_java_dir(&java_dir));
    Ok(())
  }
}

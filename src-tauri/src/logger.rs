use std::{ fs::{ self, File }, io::Write, path::PathBuf, sync::Mutex };

use chrono::Utc;
use flate2::{ write::GzEncoder, Compression };
use log::{ LevelFilter, Record };
use log4rs::{
  append::{ console::ConsoleAppender, file::FileAppender, Append },
  config::{ Appender, Root },
  encode::{ pattern::PatternEncoder, writer::simple::SimpleWriter, Encode },
  filter::{ Filter, Response },
  Config,
};

#[derive(Debug)]
struct LogLevelFilter(LevelFilter);

impl Filter for LogLevelFilter {
  fn filter(&self, record: &Record) -> Response {
    if record.level() <= self.0 { Response::Neutral } else { Response::Reject }
  }
}

#[derive(Debug)]
struct ModuleFilter(String);

impl Filter for ModuleFilter {
  fn filter(&self, record: &Record) -> Response {
    if let Some(path) = record.module_path() {
      if path == self.0 || path.starts_with(&format!("{}::", self.0)) {
        return Response::Reject;
      }
    }
    Response::Neutral
  }
}

static CALLBACKS: Mutex<Vec<Box<dyn (Fn(&str) -> Result<(), Box<dyn std::error::Error>>) + Send + Sync>>> = Mutex::new(vec![]);

#[derive(Debug)]
pub struct LauncherAppender {
  pub encoder: Box<dyn Encode>,
}

impl LauncherAppender {
  pub fn new(encoder: Box<dyn Encode>) -> Self {
    Self {
      encoder,
    }
  }

  pub fn add_callback(callback: Box<dyn (Fn(&str) -> Result<(), Box<dyn std::error::Error>>) + Send + Sync>) {
    CALLBACKS.lock().unwrap().push(callback);
  }
}

impl Append for LauncherAppender {
  fn append(&self, record: &Record) -> anyhow::Result<()> {
    let mut writer = SimpleWriter(vec![]);
    self.encoder.encode(&mut writer, &record)?;
    let msg = writer.0
      .into_iter()
      .map(|x| x as char)
      .collect::<String>();

    let callbacks = CALLBACKS.lock().unwrap();
    for callback in callbacks.iter() {
      callback(&msg).unwrap();
    }

    Ok(())
  }

  fn flush(&self) {}
}

pub fn setup_logger(logs_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  let date = Utc::now().format("%Y-%m-%d").to_string();
  let latest_log = logs_dir.join("latest.log");
  let gzipped_log = {
    let mut i = 0;
    loop {
      let gzipped_log = logs_dir.join(format!("{date}-{i}.log.gz"));
      if !gzipped_log.exists() {
        break gzipped_log;
      }
      i += 1;
    }
  };

  if latest_log.exists() {
    if let Ok(bytes) = fs::read(&latest_log) {
      let mut encoder = GzEncoder::new(File::create(gzipped_log)?, Compression::default());
      encoder.write_all(&bytes)?;
      encoder.finish()?;
    }
    fs::remove_file(&latest_log)?;
  }

  let console_encoder = PatternEncoder::new("[{d(%H:%M:%S)}] [{M}/{h({l})}]: {m}{n}");
  let launcher_encoder = PatternEncoder::new("[{d(%H:%M:%S)} {l}]: {m}{n}");
  let file_encoder = PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)} | {({l}):5.5} | {f}:{L} — {m}{n}");

  let stdout_appender = Appender::builder().build("stdout", Box::new(ConsoleAppender::builder().encoder(Box::new(console_encoder)).build()));
  let launcher_appender = Appender::builder()
    .filter(Box::new(ModuleFilter("tao".to_string())))
    .filter(Box::new(LogLevelFilter(LevelFilter::Info)))
    .build("launcher", Box::new(LauncherAppender::new(Box::new(launcher_encoder))));

  let file_appender = FileAppender::builder().encoder(Box::new(file_encoder)).build(latest_log)?;

  let root = Root::builder().appender("stdout").appender("launcher").appender("log_file").build(LevelFilter::Debug);

  let config = Config::builder()
    .appender(stdout_appender)
    .appender(launcher_appender)
    .appender(Appender::builder().build("log_file", Box::new(file_appender)))
    .build(root)
    .expect("Failed to create log4rs config");
  log4rs::init_config(config)?;

  Ok(())
}

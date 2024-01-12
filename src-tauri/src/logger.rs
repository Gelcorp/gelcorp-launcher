use std::{ sync::Mutex, io::{ BufReader, BufRead }, fs::File, path::PathBuf };

use chrono::Utc;
use log::{ LevelFilter, Record };
use log4rs::{
  append::{
    Append,
    rolling_file::{ policy::compound::{ trigger::Trigger, CompoundPolicy, roll::fixed_window::FixedWindowRoller }, LogFile, RollingFileAppender },
    console::ConsoleAppender,
  },
  encode::{ Encode, writer::simple::SimpleWriter, pattern::PatternEncoder },
  filter::{ Filter, Response },
  config::{ Appender, Root },
  Config,
};

#[derive(Debug)]
struct LogLevelFilter(LevelFilter);

impl Filter for LogLevelFilter {
  fn filter(&self, record: &Record) -> Response {
    if record.level() <= self.0 { Response::Neutral } else { Response::Reject }
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

#[derive(Debug)]
struct StartupTrigger(Mutex<bool>);

impl StartupTrigger {
  fn new() -> Self {
    Self(Mutex::new(false))
  }
}

impl Trigger for StartupTrigger {
  fn trigger(&self, file: &LogFile) -> anyhow::Result<bool> {
    let mut ran = self.0.lock().unwrap();
    if *ran {
      Ok(false)
    } else {
      *ran = true;
      let buf = BufReader::new(File::open(file.path())?);
      Ok(buf.lines().count() > 1)
    }
  }
}

pub fn setup_logger(logs_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  let console_encoder = PatternEncoder::new("[{d(%H:%M:%S)}] [{M}/{h({l})}]: {m}{n}");
  let launcher_encoder = PatternEncoder::new("[{d(%H:%M:%S)} {l}]: {m}{n}");
  let file_encoder = PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)} | {({l}):5.5} | {f}:{L} — {m}{n}");

  let stdout_appender = Appender::builder().build("stdout", Box::new(ConsoleAppender::builder().encoder(Box::new(console_encoder)).build()));
  let launcher_appender = Appender::builder()
    .filter(Box::new(LogLevelFilter(LevelFilter::Info)))
    .build("launcher", Box::new(LauncherAppender::new(Box::new(launcher_encoder))));

  // TODO: use game dir
  let date = Utc::now().format("%Y-%m-%d").to_string();
  let latest_log = logs_dir.join("latest.log");
  let gzipped_log = {
    let path = logs_dir.join(format!("{date}-{{}}.log.gz"));
    path.to_str().unwrap().to_string()
  };

  let file = RollingFileAppender::builder()
    .encoder(Box::new(file_encoder))
    .build(
      latest_log,
      Box::new(CompoundPolicy::new(Box::new(StartupTrigger::new()), Box::new(FixedWindowRoller::builder().build(&gzipped_log, 10)?)))
    )
    .unwrap();

  let root = Root::builder().appender("stdout").appender("launcher").appender("log_file").build(LevelFilter::Debug);

  let config = Config::builder()
    .appender(stdout_appender)
    .appender(launcher_appender)
    .appender(Appender::builder().build("log_file", Box::new(file)))
    .build(root)
    .expect("Failed to create log4rs config");
  log4rs::init_config(config)?;

  Ok(())
}

use std::sync::Mutex;

use log4rs::{ append::Append, encode::{ Encode, writer::simple::SimpleWriter } };

static CALLBACKS: Mutex<Vec<Box<dyn (Fn(&str) -> Result<(), Box<dyn std::error::Error>>) + Send + Sync>>> = Mutex::new(vec![]);

#[derive(Debug)]
pub struct LauncherAppender {
  pub encoder: Box<dyn Encode>,
}

impl LauncherAppender {
  pub fn add_callback(callback: Box<dyn (Fn(&str) -> Result<(), Box<dyn std::error::Error>>) + Send + Sync>) {
    CALLBACKS.lock().unwrap().push(callback);
  }
}

impl Append for LauncherAppender {
  fn append(&self, record: &log::Record) -> anyhow::Result<()> {
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

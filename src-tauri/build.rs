use std::path::Path;

use dotenv_build::Config;

fn main() {
  // Load dotenv files
  let mut configs = vec![];
  if cfg!(debug_assertions) {
    configs.push(make_dotenv_config(".env.local"));
  } else {
    configs.push(make_dotenv_config(".env.production"));
  }
  configs.push(make_dotenv_config(".env"));
  dotenv_build::output_multiple(configs).unwrap();

  tauri_build::build();
}

fn make_dotenv_config(path: &str) -> Config<'_> {
  Config {
    filename: Path::new(path),
    ..Default::default()
  }
}

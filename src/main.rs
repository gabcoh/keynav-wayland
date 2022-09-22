// TODO: Remove this allow and tempfile crate
// Allow single character names so clippy doesn't lint on x, y, r, g, b, which
// are reasonable variable names in this domain.
#![allow(clippy::many_single_char_names)]

use std::env;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

use log::info;

use keynav_wayland::app::AppRunner;
use keynav_wayland::config::*;

fn main() {
    env_logger::init();

    let mut file_or_error = match env::var_os("XDG_CONFIG_HOME") {
        Some(path) => {
            File::open(Path::new(&path).join("keynav/keynavrc")).map_err(|err| err.to_string())
        }
        None => 
            env::var_os("HOME")
            .ok_or("Neither $XDG_CONFIG_HOME nor $HOME set; cannot find a config file".into())
            .and_then(|path| {
                File::open(Path::new(&path).join(".config/keynav/keynavrc")).map_err(|err| err.to_string())
            })
    };

    let config =
        match file_or_error.as_mut() {
        Ok(file) => {
            parse_config_file(file).map_err(|err| info!("{}", err)).unwrap_or(default_config())
        }
        Err(err) => {
            info!("{}", err);
            default_config()
        }
    };

    let mut app = AppRunner::init(config).unwrap();

    while app.pump() {}
}

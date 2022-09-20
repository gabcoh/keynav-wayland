// TODO: Remove this allow and tempfile crate
// Allow single character names so clippy doesn't lint on x, y, r, g, b, which
// are reasonable variable names in this domain.
#![allow(clippy::many_single_char_names)]

use keynav_wayland::app::AppRunner;
use keynav_wayland::config::*;

fn main() {
    env_logger::init();

    let mut app = AppRunner::init(default_config()).unwrap();

    while app.pump() {}
}

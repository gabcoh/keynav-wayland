// xkbcommon made me do it
#![allow(non_upper_case_globals)]

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::vec::Vec;

use xkbcommon::xkb::*;

#[derive(Debug, PartialEq, Clone)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    // TODO: wheel?
}

impl MouseButton {
    pub fn to_code(&self) -> u32 {
        // See linux/input-event-code.h
        match self {
            Self::Left => 0x110,
            Self::Right => 0x111,
            Self::Middle => 0x112,
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s.parse::<u32>() {
            Ok(1) => Ok(Self::Left),
            Ok(2) => Ok(Self::Right),
            Ok(3) => Ok(Self::Middle),
            Ok(_) => Err("Value is out of range of MouseButton".into()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum KeynavAction {
    // Cuts and moves
    // Note: windowzoom is not implemented as I don't know how to get
    // information about the "current application window" in wayland
    CutRight(Option<f64>),
    CutLeft(Option<f64>),
    CutUp(Option<f64>),
    CutDown(Option<f64>),
    MoveRight(Option<f64>),
    MoveLeft(Option<f64>),
    MoveUp(Option<f64>),
    MoveDown(Option<f64>),
    CursorZoom { width: u32, height: u32 },

    // TODO: Grid commands
    // Mouse commands:
    Warp,
    Click(Option<MouseButton>),
    DoubleClick(Option<MouseButton>),
    // TODO: Add modifier keys
    DragButton(MouseButton),

    // TODO: Miscalenous commands
    End,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Config {
    pub mappings: HashMap<(ModMask, Keysym), Vec<KeynavAction>>,
}
impl Config {
    pub fn from_raw_config(raw_config: &RawConfig, keymap: &Keymap) -> Result<Self, String> {
        let mut mappings: HashMap<(ModMask, Keysym), Vec<KeynavAction>> = HashMap::new();
        for (raw_keys, actions) in &raw_config.mappings {
            let mut modmask: ModMask = 0;
            let mut key = None;
            for raw_key in raw_keys {
                let maybe_mod_index = keymap.mod_get_index(raw_key);
                if maybe_mod_index == MOD_INVALID {
                    let maybe_keysym = keysym_from_name(&raw_key, KEYSYM_NO_FLAGS);
                    if maybe_keysym == KEY_NoSymbol {
                        return Err(format!(
                            "String '{}' is not recognized as mod or normal key",
                            raw_key
                        ));
                    } else if key.is_some() {
                        return Err(format!(
                            "Tried to add additional non modifier key to mapping: {}",
                            raw_key
                        ));
                    } else {
                        key = Some(maybe_keysym);
                    }
                } else {
                    modmask |= maybe_mod_index;
                }
            }
            match key {
                Some(keysym) => {
                    mappings.insert((modmask, keysym), actions.clone());
                }
                None => {
                    // TODO: Give more info about where error occurs.
                    return Err("No non modifier keys found for mapping".into());
                }
            }
        }
        Ok(Config { mappings })
    }
}
#[derive(Debug, PartialEq, Clone)]
pub struct RawConfig {
    pub mappings: Vec<(Vec<String>, Vec<KeynavAction>)>,
}

pub fn default_config() -> RawConfig {
    RawConfig {
        // TODO: Fix default config
        mappings: vec![
            (vec!["h".into()], vec![KeynavAction::CutLeft(None)]),
            (vec!["j".into()], vec![KeynavAction::CutDown(None)]),
            (vec!["k".into()], vec![KeynavAction::CutUp(None)]),
            (vec!["l".into()], vec![KeynavAction::CutRight(None)]),
            (
                vec!["Shift".into(), "h".into()],
                vec![KeynavAction::MoveLeft(None)],
            ),
            (
                vec!["Shift".into(), "j".into()],
                vec![KeynavAction::MoveDown(None)],
            ),
            (
                vec!["Shift".into(), "k".into()],
                vec![KeynavAction::MoveUp(None)],
            ),
            (
                vec!["Shift".into(), "l".into()],
                vec![KeynavAction::MoveRight(None)],
            ),
            (
                vec!["semicolon".into()],
                vec![KeynavAction::CursorZoom {
                    width: 100,
                    height: 100,
                }],
            ),
            (
                vec!["Return".into()],
                vec![
                    KeynavAction::Warp,
                    KeynavAction::Click(Some(MouseButton::Left)),
                    KeynavAction::End,
                ],
            ),
            (vec!["Escape".into()], vec![KeynavAction::End]),
        ],
    }
}

pub fn parse_key(raw: &str) -> Result<Keysym, String> {
    match keysym_from_name(raw, 0) {
        KEY_NoSymbol => Err("Symbol not recognized".to_string()),
        key => Ok(key),
    }
}
pub fn parse_actions(raw: &str) -> Result<Vec<KeynavAction>, String> {
    raw.split(",")
        .map(|x| {
            let scrutinee: Vec<&str> = x.trim().split_whitespace().collect();
            match scrutinee[..] {
                ["cut-right"] => Ok(KeynavAction::CutRight(None)),
                ["cut-right", v] if v.parse::<f64>().is_ok() => {
                    Ok(KeynavAction::CutRight(Some(v.parse::<f64>().unwrap())))
                }
                ["cut-left"] => Ok(KeynavAction::CutLeft(None)),
                ["cut-left", v] if v.parse::<f64>().is_ok() => {
                    Ok(KeynavAction::CutLeft(Some(v.parse::<f64>().unwrap())))
                }
                ["cut-up"] => Ok(KeynavAction::CutUp(None)),
                ["cut-up", v] if v.parse::<f64>().is_ok() => {
                    Ok(KeynavAction::CutUp(Some(v.parse::<f64>().unwrap())))
                }
                ["cut-down"] => Ok(KeynavAction::CutDown(None)),
                ["cut-down", v] if v.parse::<f64>().is_ok() => {
                    Ok(KeynavAction::CutDown(Some(v.parse::<f64>().unwrap())))
                }
                ["move-right"] => Ok(KeynavAction::MoveRight(None)),
                ["move-right", v] if v.parse::<f64>().is_ok() => {
                    Ok(KeynavAction::MoveRight(Some(v.parse::<f64>().unwrap())))
                }
                ["move-left"] => Ok(KeynavAction::MoveLeft(None)),
                ["move-left", v] if v.parse::<f64>().is_ok() => {
                    Ok(KeynavAction::MoveLeft(Some(v.parse::<f64>().unwrap())))
                }
                ["move-up"] => Ok(KeynavAction::MoveUp(None)),
                ["move-up", v] if v.parse::<f64>().is_ok() => {
                    Ok(KeynavAction::MoveUp(Some(v.parse::<f64>().unwrap())))
                }
                ["move-down"] => Ok(KeynavAction::MoveDown(None)),
                ["move-down", v] if v.parse::<f64>().is_ok() => {
                    Ok(KeynavAction::MoveDown(Some(v.parse::<f64>().unwrap())))
                }
                ["cursorzoom", width, height]
                    if width.parse::<u32>().is_ok() && height.parse::<u32>().is_ok() =>
                {
                    Ok(KeynavAction::CursorZoom {
                        width: width.parse::<u32>().unwrap(),
                        height: height.parse::<u32>().unwrap(),
                    })
                }

                ["warp"] => Ok(KeynavAction::Warp),
                ["click"] => Ok(KeynavAction::Click(None)),
                ["click", v] if MouseButton::parse(v).is_ok() => {
                    Ok(KeynavAction::Click(Some(MouseButton::parse(v).unwrap())))
                }
                ["doubleclick"] => Ok(KeynavAction::DoubleClick(None)),
                ["doubleclick", v] if MouseButton::parse(v).is_ok() => Ok(
                    KeynavAction::DoubleClick(Some(MouseButton::parse(v).unwrap())),
                ),
                ["drag", v] if MouseButton::parse(v).is_ok() => {
                    Ok(KeynavAction::DragButton(MouseButton::parse(v).unwrap()))
                }

                ["end"] => Ok(KeynavAction::End),
                _ => Err(format!(
                    "Did not recognize \"{}\" as action (double check arguments)",
                    x
                )),
            }
        })
        .collect()
}

/*
The config file format is defined as a collection of lines separated by
newlines, where each line is either blank (containing any number of whitespace
characters other than '\n'), or a definition. A definition is a keyname followed
by whitespace followed by a nonempty comma separated list of actions.
*/
fn parse_config(contents: String) -> Result<RawConfig, String> {
    let mut mappings: Vec<(Vec<String>, Vec<KeynavAction>)> = Vec::new();
    let mut line_num = 1;
    for line in contents.split('\n') {
        match line.trim() {
            "" => {}
            line => {
                if line.chars().nth(0).unwrap_or('_') != '#' {
                    match line.find(char::is_whitespace) {
                        None => {
                            return Err(format!("Error on line {}: Line is not empty, but does not have two whitespace separated sections", line_num));
                        }
                        Some(i) => {
                            let (keys, actions) = line.split_at(i);
                            let keys = keys.split("+").map(String::from).collect();
                            let actions = parse_actions(actions.trim())
                                .map_err(|err| format!("Error on line {}: {}", line_num, err))?;
                            mappings.push((keys, actions));
                        }
                    }
                }
            }
        }
        line_num = line_num + 1;
    }

    Ok(RawConfig { mappings })
}
pub fn parse_config_file(config: &mut File) -> Result<RawConfig, String> {
    let mut contents = String::new();
    config
        .read_to_string(&mut contents)
        .map_err(|err| err.to_string())?;

    parse_config(contents)
}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn basic_parse() {
        let config = "\
h cut-left
j cut-down
k cut-up
l cut-right

Shift+h move-left
Shift+j move-down
Shift+k move-up
Shift+l move-right

semicolon cursorzoom 100 100
Return warp, click 1,end
Escape end";

        let expected = Ok(RawConfig {
            mappings: vec![
                (vec!["h".into()], vec![KeynavAction::CutLeft(None)]),
                (vec!["j".into()], vec![KeynavAction::CutDown(None)]),
                (vec!["k".into()], vec![KeynavAction::CutUp(None)]),
                (vec!["l".into()], vec![KeynavAction::CutRight(None)]),
                (
                    vec!["Shift".into(), "h".into()],
                    vec![KeynavAction::MoveLeft(None)],
                ),
                (
                    vec!["Shift".into(), "j".into()],
                    vec![KeynavAction::MoveDown(None)],
                ),
                (
                    vec!["Shift".into(), "k".into()],
                    vec![KeynavAction::MoveUp(None)],
                ),
                (
                    vec!["Shift".into(), "l".into()],
                    vec![KeynavAction::MoveRight(None)],
                ),
                (
                    vec!["semicolon".into()],
                    vec![KeynavAction::CursorZoom {
                        width: 100,
                        height: 100,
                    }],
                ),
                (
                    vec!["Return".into()],
                    vec![
                        KeynavAction::Warp,
                        KeynavAction::Click(Some(MouseButton::Left)),
                        KeynavAction::End,
                    ],
                ),
                (vec!["Escape".into()], vec![KeynavAction::End]),
            ],
        });
        assert_eq!(expected, parse_config(config.to_string()));
    }
}

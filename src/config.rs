// xkbcommon made me do it
#![allow(non_upper_case_globals)]

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::vec::Vec;

use xkbcommon::xkb::*;

#[derive(Debug, PartialEq, Clone)]
pub enum KeynavAction {
    NarrowRight,
    NarrowLeft,
    NarrowUp,
    NarrowDown,
    CenterCursor,
    MoveRight,
    MoveLeft,
    MoveUp,
    MoveDown,
    Click,
    Exit,
}

#[derive(Debug, PartialEq)]
pub struct Config {
    pub mappings: HashMap<Keysym, Vec<KeynavAction>>,
}

pub fn default_config() -> Config {
    Config {
        mappings: HashMap::from([
            (KEY_h, vec![KeynavAction::NarrowLeft]),
            (KEY_j, vec![KeynavAction::NarrowDown]),
            (KEY_k, vec![KeynavAction::NarrowUp]),
            (KEY_l, vec![KeynavAction::NarrowRight]),
            (KEY_H, vec![KeynavAction::MoveLeft]),
            (KEY_J, vec![KeynavAction::MoveDown]),
            (KEY_K, vec![KeynavAction::MoveUp]),
            (KEY_L, vec![KeynavAction::MoveRight]),
            (KEY_semicolon, vec![KeynavAction::CenterCursor]),
            (KEY_Return, vec![KeynavAction::Click, KeynavAction::Exit]),
            (KEY_Escape, vec![KeynavAction::Exit]),
        ]),
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
        .map(|x| match x.trim() {
            "narrow_right" => Ok(KeynavAction::NarrowRight),
            "narrow_left" => Ok(KeynavAction::NarrowLeft),
            "narrow_up" => Ok(KeynavAction::NarrowUp),
            "narrow_down" => Ok(KeynavAction::NarrowDown),
            "center_cursor" => Ok(KeynavAction::CenterCursor),
            "click" => Ok(KeynavAction::Click),
            "exit" => Ok(KeynavAction::Exit),
            x => Err(format!("Did not recognize \"{}\" as action", x)),
        })
        .collect()
}

/*
The config file format is defined as a collection of lines separated by
newlines, where each line is either blank (containing any number of whitespace
characters other than '\n'), or a definition. A definition is a keyname followed
by whitespace followed by a nonempty comma separated list of actions.
*/
fn parse_config(contents: String) -> Result<Config, String> {
    let mut mappings: HashMap<Keysym, Vec<KeynavAction>> = HashMap::new();
    let mut line_num = 1;
    for line in contents.split('\n') {
        match line.trim() {
            "" => {}
            line => match line.find(char::is_whitespace) {
                None => {
                    return Err(format!("Error on line {}: Line is not empty, but does not have two whitespace separated sections", line_num));
                }
                Some(i) => {
                    let (key, actions) = line.split_at(i);
                    let key = parse_key(key.trim())
                        .map_err(|err| format!("Error on line {}: {}", line_num, err))?;
                    let actions = parse_actions(actions.trim())
                        .map_err(|err| format!("Error on line {}: {}", line_num, err))?;
                    mappings.insert(key, actions);
                }
            },
        }
        line_num = line_num + 1;
    }

    Ok(Config { mappings })
}
fn parse_config_file(config: &mut File) -> Result<Config, String> {
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
h narrow_right
j narrow_down
k narrow_up
l narrow_right

Escape exit
Return click,exit";

        let expected = Ok(Config {
            mappings: HashMap::from([
                (KEY_h, vec![KeynavAction::NarrowRight]),
                (KEY_j, vec![KeynavAction::NarrowDown]),
                (KEY_k, vec![KeynavAction::NarrowUp]),
                (KEY_l, vec![KeynavAction::NarrowRight]),
                (KEY_Escape, vec![KeynavAction::Exit]),
                (KEY_Return, vec![KeynavAction::Click, KeynavAction::Exit]),
            ]),
        });
        assert_eq!(expected, parse_config(config.to_string()));
    }
}

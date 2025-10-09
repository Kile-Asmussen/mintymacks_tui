use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    process::{ExitCode, exit},
    time::Duration,
};

use clap::{Parser, Subcommand};
use indexmap::IndexMap;
use mintymacks::notation::uci::{
    engine::{EngineOption, OptionType, SpinType, UciEngine},
    gui::UciGui,
};
use serde::{Deserialize, Serialize, de::Visitor};
use tokio::{
    fs::File,
    io::{
        AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter, stderr,
        stdin, stdout,
    },
    select,
    time::sleep,
};

use crate::{
    Runnable,
    engine::{EngineDetails, EngineHandle},
    player::EnginePlayer,
};

#[derive(Serialize, Deserialize)]
pub enum Profile {
    #[serde(untagged)]
    Player(PlayerProfile),
    #[serde(untagged)]
    Engine(EngineProfile),
}

#[derive(Serialize, Deserialize)]
pub struct PlayerProfile {
    pub human: PlayerMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerMetadata {
    pub name: String,
    pub title: String,
    pub elo: i32,
}

#[derive(Serialize, Deserialize)]
pub struct EngineProfile {
    pub engine: EngineMetadata,
    pub options: IndexMap<String, OptSet>,
}

#[derive(Serialize, Deserialize)]
pub struct EngineMetadata {
    pub name: String,
    pub author: String,
    pub command: (PathBuf, Vec<String>),
    #[serde(default)]
    pub log: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptSet {
    Check(bool),
    String(String),
    Spin(i64),
}

impl EngineMetadata {
    pub fn engine_profile_toml(&self, options: &IndexMap<String, EngineOption>) -> String {
        let mut res = String::new();

        res += "[engine]\n";
        res += &toml::to_string(self).expect("Unable to render TOML");

        res += "\n[options]\n";

        for (k, v) in options {
            match &v.option_type {
                OptionType::Check(ot) => {
                    if let Some(val) = ot.value {
                        res += &format!("{k} = {val} # true or false, default {}\n", ot.default);
                    } else {
                        res += &format!("# {k} = {0} # true or false, default {0}\n", ot.default);
                    }
                }
                OptionType::Spin(ot) => {
                    let SpinType {
                        min,
                        max,
                        default,
                        value,
                    } = *ot;

                    if let Some(val) = value {
                        res +=
                            &format!("{k} = {val} # between {min} and {max}, default {default}\n",);
                    } else {
                        res += &format!(
                            "# {k} = {default} # between {min} and {max}, default {default}\n"
                        );
                    }
                }
                OptionType::Combo(ot) => {
                    let default = &ot.default;
                    let variants = ot
                        .variants
                        .iter()
                        .map(|s| format!("\"{s}\""))
                        .collect::<Vec<_>>()
                        .join(", ");

                    if let Some(val) = &ot.value {
                        res += &format!(
                            "{k} = \"{val}\" # default \"{default}\", can be one of {variants}\n",
                        );
                    } else {
                        res += &format!(
                            "# {k} = \"{default}\" # default \"{default}\", can be one of {variants}\n",
                        );
                    }
                }
                OptionType::String(ot) => {
                    if let Some(val) = &ot.value {
                        res += &format!("{k} = \"{val}\"\n# ^^^^ default \"{}\"\n", ot.default);
                    } else {
                        res += &format!("# {k} = \"{0}\"\n", ot.default);
                    }
                }
                OptionType::Button(_) => continue,
            };
            res += "\n";
        }

        res
    }
}

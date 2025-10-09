use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use mintymacks::{
    model::{ChessPiece, moves::PseudoMove},
    notation::uci::{engine::EngineOption, gui::GoCommand},
};

use crate::engine::EngineHandle;

pub struct EnginePlayer {
    pub id: String,
    pub path: PathBuf,

    pub name: Option<String>,
    pub author: Option<String>,

    pub stop_after: Duration,
    pub go: GoCommand,

    pub settings: BTreeMap<String, EngineOption>,

    pub program: Option<EngineHandle>,
}

pub struct HumanPlayer {
    pub name: String,
    pub elo: Option<u16>,
    pub title: Option<Title>,
}

pub enum Title {
    CM,
    FM,
    IM,
    GM,
}

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use mintymacks::{model::{moves::PseudoMove, ChessPiece}, notation::uci::{engine::EngineOption, gui::GoCommand}};

use crate::engine::Engine;


pub struct EnginePlayer {
    pub id: String,
    pub path: PathBuf,

    pub name: Option<String>,
    pub author: Option<String>,

    pub stop_after: Duration,
    pub go: GoCommand,

    pub settings: BTreeMap<String, EngineOption>,

    pub program: Option<Engine>,
}
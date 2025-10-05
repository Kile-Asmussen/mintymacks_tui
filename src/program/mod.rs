use std::{collections::{BTreeMap, HashMap}, path::PathBuf, time::Duration};

use mintymacks::{bits::board::BitBoard, model::moves::ChessMove, notation::{algebraic::AlgebraicMove, uci::engine::EngineOption}, zobrist::{ZobHash, ZobristBoard}};

use crate::{engine::Engine, program::game::Game, settings::Settings};

pub mod game;
pub mod human_player;
pub mod enigne_player;

pub struct Program {
    white: Option<Player>,
    black: Option<Player>,
    settings: Option<Settings>,
    hasher: ZobristBoard,
    game: Option<Game>,
    state: MenuState
}

pub enum MenuState {
    MainMenu,
    Playing,
    Setup,
}

pub enum Player {
    Human(HumanPlayer),
    Engine(EnginePlayer)
}

pub struct HumanPlayer {
    pub name: String,
    pub elo: Option<u16>,
    pub title: Option<Title>,
}

pub enum Title {
    CM, FM, IM, GM
}

pub struct EnginePlayer {
    pub id: String,
    pub path: PathBuf,

    pub name: Option<String>,
    pub author: Option<String>,

    pub time_limit: Duration,

    pub settings: BTreeMap<String, EngineOption>,

    pub program: Option<Engine>,
}
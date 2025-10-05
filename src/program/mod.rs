use std::{collections::{BTreeMap, HashMap}, path::PathBuf, time::Duration};

use mintymacks::{arrays::ArrayBoard, bits::board::BitBoard, model::{castling::CastlingRights, moves::ChessMove, ColoredChessPiece, Square}, notation::{algebraic::AlgebraicMove, uci::engine::EngineOption}, zobrist::{ZobHash, ZobristBoard}};

use crate::{engine::Engine, program::{enigne_player::EnginePlayer, game::Game, human_player::HumanPlayer}, settings::Settings};

pub mod game;
pub mod human_player;
pub mod enigne_player;

pub struct Program {
    board: ArrayBoard<Option<ColoredChessPiece>>,
    setup: Option<SetupMenu>,
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
    Analyze,
}

pub struct SetupMenu {
    castling_rights: CastlingRights,
    selected_piece: Option<ColoredChessPiece>,
}

pub enum Player {
    Human(HumanPlayer),
    Engine(EnginePlayer)
}
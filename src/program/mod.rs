use std::{collections::{BTreeMap, HashMap}, path::PathBuf, time::Duration};

use mintymacks::{arrays::ArrayBoard, bits::{board::BitBoard, BoardMask}, model::{castling::CastlingRights, moves::ChessMove, ColoredChessPiece, Square}, notation::{algebraic::AlgebraicMove, uci::engine::EngineOption}, zobrist::{ZobHash, ZobristBoard}};

use crate::{engine::Engine, program::{enigne_player::EnginePlayer, game::Game, human_player::HumanPlayer}};

pub mod game;
pub mod human_player;
pub mod enigne_player;

pub struct Program {
    board: ArrayBoard<Option<ColoredChessPiece>>,
    highlight: BoardMask,
    white: Option<Player>,
    black: Option<Player>,
    hasher: ZobristBoard,
    game: Option<Game>,
    state: MenuState
}

pub enum MenuState {
    Play,
    Analyze,
}

pub struct SetupMenu {
    castling_rights: CastlingRights,
    selected_piece: Option<ColoredChessPiece>,
}

pub enum Player {
    Record(),
    Human(HumanPlayer),
    Engine(EnginePlayer)
}
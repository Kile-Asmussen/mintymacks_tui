use std::collections::HashMap;

use mintymacks::{bits::board::BitBoard, model::moves::ChessMove, notation::algebraic::AlgebraicMove, zobrist::ZobHash};

pub struct Game {
    board: BitBoard,
    seen: HashMap<ZobHash, u8>,
    moves: Vec<ChessMove>,
    game: Vec<GameMove>,
    irreversible_move: Vec<u16>,
}

pub struct GameMove {
    before: ZobHash,
    chess: ChessMove,
    algebraic: AlgebraicMove,
}
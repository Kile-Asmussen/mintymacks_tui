use std::num::NonZero;

use mintymacks::model::{
    BoardRank, ChessPiece, Dir, Square,
    moves::{ChessMove, SpecialMove},
};

#[derive(Clone, Copy, PartialEq, Eq)]
struct MoveSelect {
    origin: Option<Square>,
    destination: Option<Square>,
    promotion: Option<Square>,
}

impl MoveSelect {
    pub fn add(&mut self, sq: Square) {
        if self.origin.is_none() {
            self.origin = Some(sq)
        } else if self.destination.is_none() {
            self.destination = Some(sq)
        } else if self.promotion.is_none() {
            self.promotion = Some(sq)
        }
    }

    pub fn matches(self, mv: ChessMove) -> bool {
        Some(mv.pmv.from) == self.origin
            && (Some(mv.pmv.to) == self.destination || self.destination.is_none())
            && (mv.spc == self.promotion() || self.promotion().is_none())
    }

    pub fn promotion(self) -> Option<SpecialMove> {
        let Some(dest) = self.destination else {
            return None;
        };
        let dir = if dest.file_rank().1 == BoardRank::_8 {
            Dir::South
        } else if dest.file_rank().1 == BoardRank::_1 {
            Dir::North
        } else {
            return None;
        };

        if self.promotion == dest.go(&[]) {
            Some(SpecialMove::Promotion(ChessPiece::Queen))
        } else if self.promotion == dest.go(&[dir]) {
            Some(SpecialMove::Promotion(ChessPiece::Rook))
        } else if self.promotion == dest.go(&[dir, dir]) {
            Some(SpecialMove::Promotion(ChessPiece::Bishop))
        } else if self.promotion == dest.go(&[dir, dir, dir]) {
            Some(SpecialMove::Promotion(ChessPiece::Knight))
        } else {
            Some(SpecialMove::Null)
        }
    }
}

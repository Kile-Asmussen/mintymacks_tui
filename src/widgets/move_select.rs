use mintymacks::{
    arrays::ArrayBoard,
    bits::{BoardMask, bit},
    model::{
        BoardRank, ChessPiece, Color, ColoredChessPiece, Dir, Square,
        moves::{ChessMove, SpecialMove},
    },
    notation::MoveMatcher,
};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct MoveSelect {
    origin: Option<Square>,
    destination: Option<Square>,
    promotion: Option<Square>,
}

impl MoveSelect {
    pub fn reset(&mut self) {
        self.origin = None;
        self.destination = None;
        self.promotion = None;
    }

    pub fn add(&mut self, sq: Square) {
        if self.origin.is_none() {
            self.origin = Some(sq)
        } else if self.destination.is_none() {
            self.destination = Some(sq)
        } else if self.promotion.is_none() {
            self.promotion = Some(sq)
        }
    }

    pub fn show_promotion(
        self,
        board: &mut ArrayBoard<Option<ColoredChessPiece>>,
    ) -> Option<BoardMask> {
        use ChessPiece::*;

        let Some(dest) = self.destination else {
            return None;
        };
        let (dir, col) = if dest.file_rank().1 == BoardRank::_8 {
            (Dir::South, Color::White)
        } else if dest.file_rank().1 == BoardRank::_1 {
            (Dir::South, Color::Black)
        } else {
            return None;
        };

        board.set(dest.go(&[]).unwrap(), Some(Queen.color(col)));
        board.set(dest.go(&[dir]).unwrap(), Some(Rook.color(col)));
        board.set(dest.go(&[dir, dir]).unwrap(), Some(Bishop.color(col)));
        board.set(dest.go(&[dir, dir, dir]).unwrap(), Some(Knight.color(col)));

        Some(
            bit(dest.go(&[]))
                | bit(dest.go(&[dir]))
                | bit(dest.go(&[dir, dir]))
                | bit(dest.go(&[dir, dir, dir])),
        )
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

impl MoveMatcher for MoveSelect {
    fn matches(&self, mv: ChessMove) -> bool {
        Some(mv.pmv.from) == self.origin
            && (Some(mv.pmv.to) == self.destination || self.destination.is_none())
            && (mv.spc == self.promotion() || self.promotion().is_none())
    }
}

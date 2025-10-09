use tokio::io::{AsyncWriteExt, stdout};

use crossterm::{
    Command, cursor,
    event::{self, Event, MouseButton, MouseEventKind},
    execute, queue,
    style::{self, Stylize},
    terminal,
};
use mintymacks::{
    arrays::ArrayBoard,
    bits::BoardMask,
    model::{self, BoardFile, BoardRank, ChessPiece, Color, ColoredChessPiece, Square},
};

static mut SETUP: bool = false;

pub async fn setup() -> tokio::io::Result<()> {
    terminal::enable_raw_mode();
    let mut out = vec![];
    execute!(
        out,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        event::EnableMouseCapture,
        terminal::SetTitle("MINTYMACKS")
    );
    stdout().write(&out[..]).await?;
    stdout().flush().await?;

    unsafe {
        SETUP = true;
    }

    Ok(())
}

pub async fn teardown() -> tokio::io::Result<()> {
    if unsafe { !SETUP } {
        return Ok(());
    }

    terminal::disable_raw_mode();
    let mut out = vec![];
    execute!(
        out,
        event::DisableMouseCapture,
        cursor::Show,
        terminal::LeaveAlternateScreen,
    );
    stdout().write(&out[..]).await?;
    stdout().flush().await?;

    Ok(())
}

pub struct BoardRenderer {
    pub row: u16,
    pub col: u16,
    pub rotated: bool,
}

impl BoardRenderer {
    pub const DARK: style::Color = style::Color::Rgb {
        r: 0x77,
        g: 0x66,
        b: 0x55,
    };
    pub const LIGHT: style::Color = style::Color::Rgb {
        r: 0xCC,
        g: 0xBB,
        b: 0xAA,
    };
    pub const HIGHLIGHT: style::Color = style::Color::Rgb {
        r: 0xDD,
        g: 0x88,
        b: 0x88,
    };
    pub const GREY: style::Color = style::Color::Rgb {
        r: 0x88,
        g: 0x88,
        b: 0x88,
    };

    pub fn translate(&self, row: u16, col: u16) -> Option<Square> {
        if row < self.row || self.row + 8 * 5 <= row {
            return None;
        }

        if col < self.col || self.col + 8 * 3 <= row {
            return None;
        }

        let row_ix = (row - self.row) / 8;
        let col_ix = (col - self.col) / 8;

        let sq = Square::at(
            BoardFile::new(row_ix as i8).unwrap(),
            BoardRank::new(col_ix as i8).unwrap(),
        );

        if self.rotated {
            Some(Self::rotate(sq))
        } else {
            Some(sq)
        }
    }

    pub fn render(
        &self,
        board: &ArrayBoard<Option<ColoredChessPiece>>,
        highlight: BoardMask,
        selected: BoardMask,
    ) -> Vec<u8> {
        let mut res = vec![];

        for (mut sq, pc) in board {
            if self.rotated {
                sq = Self::rotate(sq)
            }

            let (row, col) = self.corner(sq);
            queue!(res, cursor::MoveTo(row, col));
            self.square(
                sq,
                pc,
                highlight & sq.bit() != 0,
                selected & sq.bit() != 0,
                &mut res,
            );
        }

        res
    }

    pub fn corner(&self, sq: Square) -> (u16, u16) {
        let (f, r) = sq.file_rank();
        let mut f = f.ix() as u16;
        let mut r = r.ix() as u16 / 8;
        (self.row + f * 5, self.col + (7 - r) * 3)
    }

    pub fn square(
        &self,
        sq: Square,
        pc: Option<ColoredChessPiece>,
        highlight: bool,
        selected: bool,
        res: &mut Vec<u8>,
    ) {
        let (mut fg, mut bg) = if sq.bit() & 0x55AA55AA55AA55AA != 0 {
            (Self::DARK, Self::LIGHT)
        } else {
            (Self::LIGHT, Self::DARK)
        };

        if highlight {
            fg = Self::GREY;
            bg = Self::HIGHLIGHT;
        }

        let lsq = if self.rotated { Self::rotate(sq) } else { sq };
        let line1 = if sq.file_rank().0 == BoardFile::H {
            format!("    {}", lsq.file_rank().1.digit())
                .stylize()
                .with(fg)
                .on(bg)
        } else {
            format!("     ").with(fg).on(bg)
        };

        let line3 = if sq.file_rank().1 == BoardRank::_1 {
            format!("{}    ", lsq.file_rank().0.letter())
                .stylize()
                .with(fg)
                .on(bg)
        } else {
            format!("     ").with(fg).on(bg)
        };

        let line2 = if let Some(pc) = pc {
            (if selected {
                format!(" ({}) ", Self::piece(pc.piece()))
            } else {
                format!("  {}  ", Self::piece(pc.piece()))
            })
            .stylize()
            .with(Self::color(pc.color()))
            .on(bg)
        } else if selected {
            format!("  \u{25CB}  ").stylize().with(Self::GREY).on(bg)
        } else {
            format!("     ").stylize().with(Self::GREY).on(bg)
        };

        queue!(
            res,
            style::PrintStyledContent(line1),
            cursor::MoveDown(1),
            cursor::MoveLeft(5),
            style::PrintStyledContent(line2),
            cursor::MoveDown(1),
            cursor::MoveLeft(5),
            style::PrintStyledContent(line3),
        );
    }

    pub fn rotate(sq: Square) -> Square {
        Square::new(63 - sq.ix()).unwrap()
    }

    pub fn color(c: Color) -> style::Color {
        match c {
            Color::White => style::Color::Rgb {
                r: 0xFF,
                g: 0xFF,
                b: 0xFF,
            },
            Color::Black => style::Color::Rgb { r: 0, g: 0, b: 0 },
        }
    }

    pub fn piece(pc: ChessPiece) -> char {
        match pc {
            ChessPiece::Pawn => '\u{265F}',
            ChessPiece::Knight => '\u{265E}',
            ChessPiece::Bishop => '\u{265D}',
            ChessPiece::Rook => '\u{265C}',
            ChessPiece::Queen => '\u{265B}',
            ChessPiece::King => '\u{265A}',
        }
    }
}

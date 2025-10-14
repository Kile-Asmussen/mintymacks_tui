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
    model::{
        self, BoardFile, BoardRank, ChessPiece, Color, ColoredChessPiece, Dir, Square,
        moves::{ChessMove, SpecialMove},
    },
    notation::MoveMatcher,
};

pub mod board;
pub mod move_select;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRenderer {
    pub row: u16,
    pub col: u16,
    pub style: style::ContentStyle,
}

impl TextRenderer {
    pub fn render(self, data: &str) -> Vec<u8> {
        let mut res = vec![];

        queue!(res, cursor::MoveTo(self.col, self.row));

        for line in data.lines() {
            let mut line = line.trim_end().stylize();
            *line.style_mut() = self.style;
            queue!(
                res,
                style::PrintStyledContent(line),
                cursor::MoveDown(1),
                cursor::MoveToColumn(self.col)
            );
        }

        res
    }
}

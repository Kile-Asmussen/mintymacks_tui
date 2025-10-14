use std::{
    collections::VecDeque,
    path::PathBuf,
    process::{self, ExitCode},
    time::Duration,
};

use clap::Parser;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyModifiers},
    queue,
    style::{self, ContentStyle, Stylize},
    terminal,
};
use mintymacks::{
    arrays::ArrayBoard,
    bits::{board::BitBoard, two_bits},
    eprintln_async,
    game::{FatMove, GameReview, GameState},
    model::{
        Color, ColoredChessPiece, Victory,
        castling::CastlingMove,
        moves::{ChessMove, SpecialMove},
    },
    notation::pgn::{MovePair, PGN, PGNTags, load_pgn_file},
};
use tokio::{
    io::{AsyncWriteExt, stdout},
    select,
    time::sleep,
};
use tokio_stream::StreamExt;

use crate::{
    Runnable,
    widgets::{self, TextRenderer, board::BoardRenderer},
};

#[derive(Parser)]
pub struct ReviewGame {
    #[clap()]
    file: PathBuf,
}

impl Runnable for ReviewGame {
    async fn run(self) -> tokio::io::Result<()> {
        let pgns = load_pgn_file(&String::from_utf8_lossy_owned(
            tokio::fs::read(&self.file).await?,
        ));

        let mut reviews = vec![];

        for (ix, pgn) in pgns.into_iter().enumerate() {
            let ix = ix + 1;
            let game = GameState::from_pgn(&pgn);

            let game = match game {
                Err(s) => {
                    eprintln_async!("Error in parsing PGN game #{}: {}", ix, s).await;
                    ExitCode::FAILURE.exit_process();
                }
                Ok(g) => g,
            };

            reviews.push(GameReview::new(&game, pgn.headers.clone()))
        }

        if reviews.is_empty() {
            eprintln_async!("No games found").await;
            ExitCode::FAILURE.exit_process();
        }

        let mut gr = GameReviewer {
            file: self.file.clone(),
            reviews,
            index: 0,
            rotated: false,
            offset: 0,
        };

        gr.mainloop().await?;

        ExitCode::SUCCESS.exit_process();
    }
}

pub struct GameReviewer {
    pub file: PathBuf,
    pub reviews: Vec<GameReview>,
    pub index: usize,
    pub rotated: bool,
    pub offset: usize,
}

impl GameReviewer {
    pub fn board_render(&self) -> BoardRenderer {
        BoardRenderer {
            col: 3,
            row: 2,
            rotated: self.rotated,
        }
    }

    pub fn title_renderer(&self) -> TextRenderer {
        TextRenderer {
            col: 3,
            row: 1,
            style: ContentStyle::new(),
        }
    }

    pub fn metadata_renderer(&self) -> TextRenderer {
        TextRenderer {
            col: 3 + 5 * 8 + 1 + 15 + 1,
            row: 2,
            style: ContentStyle::new(),
        }
    }

    pub fn moves_renderer(&self) -> TextRenderer {
        TextRenderer {
            col: 3 + 5 * 8 + 1,
            row: 2,
            style: ContentStyle::new(),
        }
    }

    pub const GREY: style::Color = style::Color::Rgb {
        r: 0x88,
        g: 0x88,
        b: 0x88,
    };

    pub fn future_moves_renderer(&self) -> TextRenderer {
        TextRenderer {
            col: 3 + 5 * 8 + 1,
            row: 2 + 14,
            style: ContentStyle::new().with(Self::GREY),
        }
    }

    pub fn reminder_renderer() -> TextRenderer {
        TextRenderer {
            row: 2 + 8 * 3 + 1,
            col: 3,
            style: ContentStyle::new().with(Self::GREY),
        }
    }

    pub fn current(&self) -> &GameReview {
        &self.reviews[self.index]
    }

    pub fn current_mut(&mut self) -> &mut GameReview {
        &mut self.reviews[self.index]
    }

    pub fn go_prev_game(&mut self) {
        if self.index > 1 {
            self.current_mut().to_start();
            self.index -= 1;
            self.offset = 0;
        }
    }

    pub fn go_next_game(&mut self) {
        if self.index + 1 < self.reviews.len() {
            self.current_mut().to_start();
            self.index += 1;
            self.offset = 0;
        }
    }

    pub async fn render(&self) -> tokio::io::Result<()> {
        let board = self.current().cursor.render();
        let highlight = match self.current().past.back() {
            None => 0,
            Some(fm) => {
                if let Some(SpecialMove::CastlingEastward) = fm.chessmove.spc {
                    let cd = self
                        .current()
                        .cursor
                        .metadata
                        .castling_details
                        .eastward
                        .reify(fm.chessmove.cpc.color());
                    cd.king_move.bits() | cd.rook_move.bits()
                } else if let Some(SpecialMove::CastlingWestward) = fm.chessmove.spc {
                    let cd = self
                        .current()
                        .cursor
                        .metadata
                        .castling_details
                        .westward
                        .reify(fm.chessmove.cpc.color());
                    cd.king_move.bits() | cd.rook_move.bits()
                } else {
                    fm.chessmove.pmv.bits()
                }
            }
        };

        let mut res = vec![];

        queue!(res, terminal::Clear(terminal::ClearType::Purge));

        res.append(&mut self.board_render().render(&board, highlight, 0));

        res.append(&mut self.title_renderer().render(&format!(
            "Game {} of {} in file {}",
            self.index + 1,
            self.reviews.len(),
            self.file.to_string_lossy(),
        )));

        let mut past_moves = self
            .current()
            .past_pgn()
            .iter()
            .map(|mp| mp.to_string())
            .collect::<Vec<_>>();
        past_moves = past_moves.split_off(past_moves.len().saturating_sub(13));
        past_moves.push(String::from_iter(['\u{2500}'; 15]));
        while past_moves.len() < 14 {
            past_moves.insert(0, String::new());
        }

        let mut future_moves = self
            .current()
            .future_pgn()
            .iter()
            .map(|mp| mp.to_string())
            .collect::<Vec<_>>();
        future_moves.truncate(10);

        res.append(
            &mut self
                .moves_renderer()
                .render(&format!("{}", past_moves.join("\n"))),
        );

        res.append(
            &mut self
                .future_moves_renderer()
                .render(&format!("{}", future_moves.join("\n"))),
        );

        res.append(&mut self.metadata_renderer().render(&format!("{}", {
            let mut x = String::new();
            self.current().tags.to_string(&mut x);
            x
        })));

        res.append(
            &mut Self::reminder_renderer()
                .render("[↑] and [↓]: navigate moves\n[x]: rotate board\n[Ctrl]+[←] and [Ctrl]+[→]: navigate between games\n[Ctrl]+[C] or [ESC]: Exit"),
        );

        stdout().write_all(&res[..]).await?;

        Ok(())
    }

    pub fn handle(&mut self, ev: Event) -> bool {
        match ev {
            Event::Key(key_event) if key_event.is_press() || key_event.is_repeat() => {
                match key_event.code {
                    KeyCode::Left if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.go_prev_game();
                    }
                    KeyCode::Right if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.go_next_game();
                    }
                    KeyCode::Up => {
                        self.current_mut().prev();
                    }
                    KeyCode::Down => {
                        self.current_mut().next();
                    }
                    KeyCode::Char('x') => self.rotated = !self.rotated,
                    KeyCode::Esc => return true,
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        return true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        return false;
    }

    pub async fn mainloop(&mut self) -> tokio::io::Result<()> {
        widgets::setup().await?;

        let mut event_stream = EventStream::new().fuse();

        loop {
            self.render().await?;

            let event = event_stream.next();

            select! {
                ev = event => {
                    if let Some(ev) = ev {
                        if self.handle(ev?) {
                            break;
                        }
                    }
                }
                _ = sleep(Duration::from_millis(50)) => {}
            }
        }

        widgets::teardown().await?;

        Ok(())
    }
}

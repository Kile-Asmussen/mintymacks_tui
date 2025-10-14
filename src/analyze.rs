use std::{path::PathBuf, process::ExitCode};

use clap::Parser;
use mintymacks::{
    model::Color,
    notation::pgn::{PGN, PGNTags, load_pgn_file},
};

use crate::Runnable;

#[derive(Parser)]
pub struct ReviewGame {
    #[clap()]
    file: PathBuf,
}

impl Runnable for ReviewGame {
    fn tui(&self) -> bool {
        true
    }

    async fn run(self) -> tokio::io::Result<ExitCode> {
        let pgns = load_pgn_file(&String::from_utf8_lossy_owned(
            tokio::fs::read(self.file).await?,
        ));

        let mut index = 0usize;

        Ok(ExitCode::SUCCESS)
    }
}

pub struct GameReviewer {
    pgns: Vec<PGN>,
    game_index: usize,
    move_index: (u16, Color),
}

impl GameReviewer {
    fn next_game(&mut self) {
        if self.game_index < self.pgns.len() {
            self.game_index += 1;
            self.move_index = (1, Color::White);
        }
    }

    fn prev_game(&mut self) {
        if self.game_index > 0 {
            self.game_index -= 1;
            self.move_index = (1, Color::White);
        }
    }

    fn headers(&self) -> &PGNTags {}
}

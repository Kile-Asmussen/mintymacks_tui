use std::{path::PathBuf, process::ExitCode};

use clap::Parser;
use mintymacks::{
    bits::board::BitBoard,
    eprintln_async,
    game::{GameReview, GameState},
    model::{Color, Victory},
    notation::pgn::{MovePair, PGN, PGNTags, load_pgn_file},
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

        let reviews = vec![];

        for (ix, pgn) in pgns.into_iter().enumerate() {
            let ix = ix + 1;
            let game = GameState::from_pgn(&pgn);

            let game = match game {
                Err(s) => {
                    eprintln_async!("Error in parsing PGN game #{}: {}", ix, s);
                    return Ok(ExitCode::FAILURE);
                }
                Ok(g) => g,
            };

            reviews.push(GameReview::new(&game))
        }

        Ok(ExitCode::SUCCESS)
    }
}

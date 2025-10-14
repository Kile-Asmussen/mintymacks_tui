use core::hash;
use std::{
    collections::{HashMap, VecDeque},
    fmt::format,
    path::PathBuf,
    process::{self, ExitCode},
    time::Duration,
};

use clap::Parser;
use crossterm::{cursor, execute, style, terminal};
use mintymacks::{
    bits::board::{self, BitBoard},
    engine::{EngineHandle, load_engine},
    game::GameState,
    model::{
        ChessPiece, Victory,
        moves::{ChessMove, PseudoMove},
    },
    notation::{
        algebraic::AlgebraicMove,
        fen::render_fen,
        pgn::{MovePair, PGN, PGNHeaders},
        uci::{
            LongAlg,
            engine::{BestMove, OptionType, UciEngine},
            gui::{GoCommand, PositionString, UciGui},
        },
    },
    profile::EngineProfile,
    zobrist::{ZobHash, ZobristBoard},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, stdin, stdout},
    select,
    time::{Instant, sleep},
};

use crate::Runnable;

#[derive(Parser)]
pub struct Faceoff {
    /// Bot profile
    #[clap(long)]
    pub white: PathBuf,

    /// Bot profile
    #[clap(long)]
    pub black: PathBuf,

    /// Turn time in miliseconds
    #[clap(long)]
    pub time: u64,

    /// Turn timeout in miliseconds
    #[clap(long)]
    pub timeout: u64,

    /// Only print the end result
    #[clap(short)]
    pub quiet: bool,
}

impl Runnable for Faceoff {
    fn tui(&self) -> bool {
        false
    }

    async fn run(self) -> tokio::io::Result<ExitCode> {
        let white_profile = tokio::fs::read(&self.white).await?;
        let black_profile = tokio::fs::read(&self.black).await?;

        let bad_file = |_| tokio::io::Error::from(tokio::io::ErrorKind::InvalidData);

        let white_profile: EngineProfile = toml::from_slice(&white_profile).map_err(bad_file)?;
        let black_profile: EngineProfile = toml::from_slice(&black_profile).map_err(bad_file)?;

        let (mut white_engine, mut white_details) = load_engine(&white_profile).await?;

        let (mut black_engine, mut black_details) = load_engine(&black_profile).await?;

        white_engine
            .interleave(
                &mut VecDeque::from([UciGui::UciNewGame()]),
                &mut vec![],
                Duration::from_millis(1000),
            )
            .await?;
        black_engine
            .interleave(
                &mut VecDeque::from([UciGui::UciNewGame()]),
                &mut vec![],
                Duration::from_millis(100),
            )
            .await?;

        let mut game = GameState::startpos();
        game.white = Some(mintymacks::profile::Profile::Engine(white_profile.clone()));
        game.black = Some(mintymacks::profile::Profile::Engine(black_profile.clone()));

        Ok(ExitCode::SUCCESS)
    }
}

async fn best_move(
    engine: &mut EngineHandle,
    move_history: &[(PseudoMove, Option<ChessPiece>)],
    time: u64,
    timeout: u64,
) -> tokio::io::Result<Option<BestMove>> {
    let mut arg = VecDeque::from([
        UciGui::Position(PositionString::Startpos(), Vec::from(move_history)),
        UciGui::Go(GoCommand::Infinite()),
    ]);
    let mut res = vec![];

    let now = Instant::now();

    let (mut ingress, mut egress) = engine.split();

    loop {
        select! {
            _ = sleep(Duration::from_millis(time) / 10) => {}
            Ok(uci) = ingress.receive() => {
                if let Some(UciEngine::BestMove(bm)) = uci {
                    return Ok(Some(bm));
                } else if let Some(uci) = uci {
                    res.push(uci);
                }
            }
            _ = egress.send(arg.front()), if !arg.is_empty() => {
                arg.pop_front();
            }
        }

        if now.elapsed() > Duration::from_millis(time) {
            arg.push_back(UciGui::Stop());
        }

        if now.elapsed() > Duration::from_millis(timeout) {
            return Ok(None);
        }
    }
}

async fn show_pgn(pgn: &PGN, clear: bool) -> tokio::io::Result<()> {
    let mut s = "\n".to_string();
    pgn.to_string(&mut s, true);
    let mut res = vec![];

    if clear {
        execute!(
            res,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
            style::Print(s)
        );
    } else {
        execute!(res, style::Print(s));
    }

    stdout().write_all(&res[..]).await?;
    stdout().flush().await?;

    Ok(())
}

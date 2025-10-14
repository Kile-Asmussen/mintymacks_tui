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
    deque,
    engine::{EngineHandle, load_engine},
    game::GameState,
    model::{
        ChessPiece, Victory,
        moves::{ChessMove, PseudoMove},
    },
    notation::{
        algebraic::AlgebraicMove,
        fen::render_fen,
        pgn::{MovePair, PGN},
        uci::{
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

        let mut ingress = vec![];
        white_engine
            .interleave_until(
                &mut deque![UciGui::UciNewGame(), UciGui::IsReady()],
                &mut ingress,
                |x| x == &UciEngine::ReadyOk(),
                Duration::from_millis(5000),
            )
            .await?;
        if ingress.last() != Some(&UciEngine::ReadyOk()) {
            return Ok(ExitCode::FAILURE);
        }

        ingress.clear();
        black_engine
            .interleave_until(
                &mut deque![UciGui::UciNewGame(), UciGui::IsReady()],
                &mut ingress,
                |x| x == &UciEngine::ReadyOk(),
                Duration::from_millis(5000),
            )
            .await?;
        if ingress.last() != Some(&UciEngine::ReadyOk()) {
            return Ok(ExitCode::FAILURE);
        }

        let mut game = GameState::startpos();
        game.white = Some(mintymacks::profile::Profile::Engine(white_profile.clone()));
        game.black = Some(mintymacks::profile::Profile::Engine(black_profile.clone()));

        let mut ponder_white = None;
        let mut move_white = (PseudoMove::NULL, None);
        let mut ponder_black = None;
        let mut move_black = (PseudoMove::NULL, None);

        let time = Duration::from_millis(self.time)
        let timeout = Duration::from_millis(self.timeout);

        loop {
            if let Some(m) = query_best_move(&mut white_engine, &mut game, time, timeout, ).await? {
                ponder_white = m.ponder;
                if let Ok(fm) = game.find_move(m.best) {
                    game.apply(fm);
                    move_white = Some(m.best);
                }
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}

async fn query_best_move(
    engine: &mut EngineHandle,
    game: &mut GameState,
    time: Duration,
    timeout: Duration,
    ponderhit: bool,
) -> tokio::io::Result<Option<BestMove>> {
    let mut arg = deque![
        UciGui::Position(game.uci_position(), game.uci_line()),
        UciGui::Go(GoCommand::Infinite()),
    ];

    let now = Instant::now();

    let (mut ingress, mut egress) = engine.split();

    loop {
        select! {
            _ = sleep(timeout / 10) => {}
            Ok(uci) = ingress.receive() => {
                if let UciEngine::BestMove(bm) = uci {
                    return Ok(Some(bm));
                }
            }
            _ = egress.send(arg.front()), if !arg.is_empty() => {
                arg.pop_front();
            }
        }

        if now.elapsed() > timeout {
            arg.push_back(UciGui::Stop());
        }

        if now.elapsed() > timeout * 2 {
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

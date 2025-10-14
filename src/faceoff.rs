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
        ChessPiece, Victory, WinReason,
        moves::{ChessMove, PseudoMove},
    },
    notation::{
        LongAlg,
        algebraic::AlgebraicMove,
        fen::render_fen,
        pgn::{MovePair, PGN},
        uci::{
            engine::{BestMove, OptionType, UciEngine},
            gui::{GoCommand, PositionString, UciGui},
        },
    },
    print_async,
    profile::EngineProfile,
    utils::{eprintln_async, println_async},
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
}

impl Runnable for Faceoff {
    async fn run(self) -> tokio::io::Result<()> {
        eprintln_async!("Loading profiles...").await;
        let white_profile = tokio::fs::read(&self.white).await?;
        let black_profile = tokio::fs::read(&self.black).await?;

        let bad_file = |_| tokio::io::Error::from(tokio::io::ErrorKind::InvalidData);

        let white_profile: EngineProfile = toml::from_slice(&white_profile).map_err(bad_file)?;
        let black_profile: EngineProfile = toml::from_slice(&black_profile).map_err(bad_file)?;

        eprintln_async!("Loading white engine...").await;
        let (mut white_engine, mut white_details) = load_engine(&white_profile).await?;
        eprintln_async!("Loading black engine...").await;
        let (mut black_engine, mut black_details) = load_engine(&black_profile).await?;

        eprintln_async!("Initializing white engine...").await;
        let mut ingress = vec![];
        white_engine
            .interleave_until(
                &mut deque![UciGui::UciNewGame(), UciGui::IsReady()],
                &mut ingress,
                |x| x == &UciEngine::ReadyOk(),
                Duration::from_millis(1000),
            )
            .await?;
        if ingress.last() != Some(&UciEngine::ReadyOk()) {
            eprintln_async!("Engine did not respond `readyok' in time.").await;
            ExitCode::FAILURE.exit_process();
        }

        eprintln_async!("Initializing black engine...").await;
        ingress.clear();
        black_engine
            .interleave_until(
                &mut deque![UciGui::UciNewGame(), UciGui::IsReady()],
                &mut ingress,
                |x| x == &UciEngine::ReadyOk(),
                Duration::from_millis(1000),
            )
            .await?;
        if ingress.last() != Some(&UciEngine::ReadyOk()) {
            eprintln_async!("Engine did not respond `readyok' in time.").await;
            ExitCode::FAILURE.exit_process();
        }

        eprintln_async!("Starting game...").await;

        let mut game = GameState::startpos();
        game.white = Some(mintymacks::profile::Profile::Engine(white_profile.clone()));
        game.black = Some(mintymacks::profile::Profile::Engine(black_profile.clone()));

        let time = Duration::from_millis(self.time);
        let timeout = Duration::from_millis(self.timeout);

        for (k, v) in game.pgn_header().0 {
            println_async!("[{k} \"{v}\"]").await;
        }
        println_async!().await;

        loop {
            if let Some(v) = game.outcome {
                println_async!("{}", v.to_string()).await;
                break;
            } else {
                println_async!().await;
            }

            print_async!("{}. ", game.board.metadata.turn).await;

            if let Some(m) = query_best_move(&mut white_engine, &mut game, time, timeout).await? {
                if let Ok(fm) = game.find_move(m.best) {
                    print_async!("{} ", game.apply(fm).unwrap().algebraic.to_string()).await;
                } else {
                    game.outcome = Some(Victory::BlackWins(WinReason::Forefeit));
                    print_async!("{}", m.best.0.longalg(m.best.1)).await;
                }
            } else {
                game.outcome = Some(Victory::BlackWins(WinReason::Time));
            }

            if let Some(v) = game.outcome {
                println_async!("{}", v.to_string()).await;
                break;
            }

            if let Some(m) = query_best_move(&mut black_engine, &mut game, time, timeout).await? {
                if let Ok(fm) = game.find_move(m.best) {
                    print_async!("{} ", game.apply(fm).unwrap().algebraic.to_string()).await;
                } else {
                    game.outcome = Some(Victory::WhiteWins(WinReason::Forefeit));
                    print_async!("{}", m.best.0.longalg(m.best.1)).await;
                }
            } else {
                game.outcome = Some(Victory::WhiteWins(WinReason::Time));
            }
        }

        ExitCode::SUCCESS.exit_process();
    }
}

async fn query_best_move(
    engine: &mut EngineHandle,
    game: &GameState,
    time: Duration,
    timeout: Duration,
) -> tokio::io::Result<Option<BestMove>> {
    let mut arg = deque![
        UciGui::Position(game.uci_position(), game.uci_line()),
        UciGui::Go(GoCommand::Infinite()),
    ];

    let now = Instant::now();

    let (mut ingress, mut egress) = engine.split();

    loop {
        select! {
            _ = sleep(time / 10) => {}
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

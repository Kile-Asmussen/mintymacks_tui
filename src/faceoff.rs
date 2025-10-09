use std::{
    collections::VecDeque,
    fmt::format,
    path::PathBuf,
    process::{self, ExitCode},
    time::Duration,
};

use clap::Parser;
use crossterm::{cursor, execute, style, terminal};
use mintymacks::{
    bits::board::{self, BitBoard},
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
};
use tokio::{
    io::{AsyncWriteExt, stdout},
    select,
    time::{Instant, sleep},
};

use crate::{
    Runnable,
    engine::{EngineDetails, EngineHandle, load_engine},
    player::EnginePlayer,
    profile::EngineProfile,
};

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

        set_ownbook(&mut white_engine, &mut white_details);
        set_ownbook(&mut black_engine, &mut black_details);

        white_engine
            .interleave(
                &mut VecDeque::from([UciGui::UciNewGame()]),
                &mut vec![],
                Duration::from_millis(100),
            )
            .await?;
        black_engine
            .interleave(
                &mut VecDeque::from([UciGui::UciNewGame()]),
                &mut vec![],
                Duration::from_millis(100),
            )
            .await?;

        let mut res = PGN {
            headers: PGNHeaders::default(),
            moves: vec![],
            end: "*".to_string(),
        };

        res.headers.white = Some(white_details.name.clone());
        res.headers.black = Some(black_details.name.clone());

        let mut board = BitBoard::startpos();
        let mut pseudomoves = vec![];
        let mut current = vec![];
        board.moves(&mut current);

        loop {
            let mut pair = MovePair {
                turn: board.metadata.turn as u64,
                white: None,
                white_nag: 0,
                white_comment: None,
                black: None,
                black_nag: 0,
                black_comment: None,
            };

            res.moves.push(pair.clone());
            show_pgn(&res).await?;

            if pair.turn > 100 {
                break;
            }

            let Some((cm, mut am, pmv)) = find_move(
                &mut board,
                &mut white_engine,
                &pseudomoves,
                &current,
                self.time,
                &mut res.end,
            )
            .await?
            else {
                break;
            };

            let (am, win) = make_moves(&mut board, cm, am, pmv, &mut pseudomoves, &mut current);

            pair.white = Some(am);
            if win.is_some() {
                res.end = winstring(win);
                break;
            }

            res.moves.pop();
            res.moves.push(pair.clone());

            show_pgn(&res).await?;

            let Some((cm, mut am, pmv)) = find_move(
                &mut board,
                &mut black_engine,
                &pseudomoves,
                &current,
                self.time,
                &mut res.end,
            )
            .await?
            else {
                break;
            };

            let (am, win) = make_moves(&mut board, cm, am, pmv, &mut pseudomoves, &mut current);

            pair.black = Some(am);
            if win.is_some() {
                res.end = winstring(win);
                break;
            }
            show(format!("Playing {}", pair.to_string())).await?;
            res.moves.pop();
            res.moves.push(pair);

            show_pgn(&res).await?;
        }

        show_pgn(&res).await?;

        Ok(ExitCode::SUCCESS)
    }
}

async fn show(mut s: String) -> tokio::io::Result<()> {
    s += "\n";
    stdout().write_all(s.as_bytes()).await?;
    stdout().flush().await?;
    Ok(())
}

fn make_moves(
    board: &mut BitBoard,
    cm: ChessMove,
    mut am: AlgebraicMove,
    pmv: LongAlg,
    pseudomoves: &mut Vec<LongAlg>,
    current: &mut Vec<ChessMove>,
) -> (AlgebraicMove, Option<Victory>) {
    board.apply(cm);
    pseudomoves.push(pmv);
    if check(&board) {
        am.check_or_mate = Some(false);
    }
    current.clear();
    board.moves(current);

    let win = Victory::determine(&board, 0, &current, 0, &hash_map! {});
    if win.is_some() && win == Some(Victory::from_color(cm.piece.color())) {
        am.check_or_mate = Some(false);
    }

    (am, win)
}

fn check(board: &BitBoard) -> bool {
    let (act, pas) = board.active_passive(board.metadata.to_move);

    let threats = pas.threats(board.metadata.to_move.opposite(), act.total, None, None);

    (act.kings & threats) != 0
}

async fn find_move(
    board: &mut BitBoard,
    engine: &mut EngineHandle,
    history: &[LongAlg],
    current: &[ChessMove],
    time: u64,
    end: &mut String,
) -> tokio::io::Result<Option<(ChessMove, AlgebraicMove, LongAlg)>> {
    let mover = board.metadata.to_move;

    let Some(BestMove { best: pmv, .. }) = best_move(engine, &history, time).await? else {
        *end = winstring(Some(Victory::from_color(mover.opposite())));
        *end += " {timeout}";
        return Ok(None);
    };

    let Some(cm) = current.iter().find(|cm| cm.simplify() == pmv) else {
        *end = winstring(Some(Victory::from_color(mover.opposite())));
        *end += &format!("\n{{illegal move {} suggested}}", pmv.0.longalg(pmv.1));
        *end += &format!("\n{{FEN {}}}", render_fen(&board, 0));
        *end += &format!(
            "\n{{Legal moves: {}}}",
            current
                .iter()
                .map(|cm| (cm.ambiguate(&board, &current), cm.simplify()))
                .map(|(a, (m, p))| format!("{} ({})", a.to_string(), m.longalg(p)))
                .collect::<Vec<_>>()
                .join(", ")
        );
        *end += &format!(
            "{{ {:?} }}",
            current.iter().find(|cm| cm.simplify().0 == pmv.0)
        );
        return Ok(None);
    };
    let cm = *cm;

    let am = cm.ambiguate(&board, &current);

    Ok(Some((cm, am, pmv)))
}

fn winstring(vic: Option<Victory>) -> String {
    match vic {
        Some(Victory::WhiteWins) => "1-0".to_string(),
        Some(Victory::BlackWins) => "0-1".to_string(),
        Some(Victory::Draw) => "1/2-1/2".to_string(),
        None => "*".to_string(),
    }
}

async fn best_move(
    engine: &mut EngineHandle,
    move_history: &[(PseudoMove, Option<ChessPiece>)],
    time: u64,
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
            _ = sleep(Duration::from_millis(time) / 100) => {}
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

        if now.elapsed() > Duration::from_millis(time) * 2 {
            return Ok(None);
        }
    }
}

async fn set_ownbook(
    engine: &mut EngineHandle,
    details: &mut EngineDetails,
) -> tokio::io::Result<()> {
    details
        .options
        .entry("OwnBook".to_string())
        .and_modify(|e| {
            if let OptionType::Check(c) = &mut e.option_type {
                c.value = Some(true)
            }
        });

    engine
        .interleave(
            &mut details.set_options(),
            &mut vec![],
            Duration::from_millis(100),
        )
        .await?;

    Ok(())
}

async fn show_pgn(pgn: &PGN) -> tokio::io::Result<()> {
    let mut s = "\n".to_string();
    pgn.to_string(&mut s, true);
    let mut res = vec![];

    execute!(
        res,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        style::Print(s)
    );

    stdout().write_all(&res[..]).await?;
    stdout().flush().await?;

    Ok(())
}

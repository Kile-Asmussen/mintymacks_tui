#![allow(unused)]
#![feature(iter_collect_into)]
#![feature(const_index)]
#![feature(const_trait_impl)]
#![feature(slice_partition_dedup)]
#![feature(default_field_values)]
#![feature(hash_map_macro)]
#![feature(try_blocks)]
#![feature(adt_const_params)]

use std::{collections::{vec_deque, VecDeque}, io::{stdout, Result, Write}, path::Path, process::Output, time::Duration};
use crossterm::{cursor, event::{self, Event, EventStream, KeyCode}, execute, style::Print, terminal};
use mintymacks::{bits::board::BitBoard, notation::uci::{engine::UciEngine, gui::UciGui}};
use tokio::{io::AsyncWriteExt, select, time::sleep};
use tokio_stream::{Stream, StreamExt};

use crate::{engine::Engine, widgets::BoardRenderer};

mod run;
mod engine;
mod settings;
mod widgets;
mod move_select;
mod player;

#[tokio::main]
pub async fn main() -> tokio::io::Result<()> {
    widgets::setup()?;

    let board_render = BoardRenderer { row: 4, col: 2, rotated: false };

    let bitboard = BitBoard::startpos();

    let mut events = EventStream::new();

    loop {
        let ev = events.next();
        let delay = sleep(Duration::from_millis(1000/60));
        select! {
            Some(ev) = ev => {
                if let Ok(ev) = ev {
                    match ev {
                        Event::Key(k) => {
                            if k.code == KeyCode::Char('q') {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ = delay => {}
        }
        let arrayboard = bitboard.render();
        let board = board_render.render(&arrayboard, 0, 0);
        tokio::io::stdout().write_all(&board).await?;
    }
    
    widgets::teardown()?;

    Ok(())
}


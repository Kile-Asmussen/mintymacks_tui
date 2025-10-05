#![allow(unused)]
#![feature(iter_collect_into)]
#![feature(const_index)]
#![feature(const_trait_impl)]
#![feature(slice_partition_dedup)]
#![feature(default_field_values)]
#![feature(hash_map_macro)]
#![feature(try_blocks)]
#![feature(adt_const_params)]
#![feature(path_add_extension)]

use std::{collections::{vec_deque, VecDeque}, io::{stdout, Result}, path::Path, process::Output, time::Duration};
use crossterm::{cursor, event, execute, style::Print, terminal};
use mintymacks::notation::uci::{engine::UciEngine, gui::UciGui};
use tokio::select;
use tokio_stream::{Stream, StreamExt};

use crate::engine::Engine;

mod run;
mod engine;
mod program;
mod settings;
mod user_input;

#[tokio::main]
pub async fn main() -> tokio::io::Result<()> {
    let mut stockfish = Engine::new(Path::new("stockfish"), &[]).await?;

    
    let (mut ucout, mut ucin) = stockfish.split();

    let mut out = VecDeque::from([UciGui::Uci()]);
    let mut res = vec![];

    loop {
        let delay = tokio::time::sleep(Duration::from_millis(1000));

        select! {
            _ = delay => {
                break;
            }
            uci = ucin.read() => {
                match uci? {
                    Some(UciEngine::UciOk()) => {
                        res.push(UciEngine::UciOk());
                        break;
                    }
                    Some(uci) => res.push(uci),
                    None => {}
                }
            }
            Ok(true) = ucout.write(out.back()), if !out.is_empty() => {
                out.pop_back();
            }
        }
    }

    stockfish.quit().await;

    for uci in res {
        println!("{:?}", uci);
    }

    Ok(())
}
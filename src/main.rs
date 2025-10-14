#![allow(unused)]
#![feature(iter_collect_into)]
#![feature(const_index)]
#![feature(const_trait_impl)]
#![feature(slice_partition_dedup)]
#![feature(default_field_values)]
#![feature(hash_map_macro)]
#![feature(try_blocks)]
#![feature(adt_const_params)]
#![allow(async_fn_in_trait)]
#![feature(push_mut)]
#![feature(format_args_nl)]
#![feature(string_from_utf8_lossy_owned)]
#![feature(exitcode_exit_method)]

use clap::{Parser, Subcommand};
use crossterm::{
    cursor,
    event::{self, Event, EventStream, KeyCode},
    execute,
    style::Print,
    terminal,
};
use mintymacks::{
    bits::board::BitBoard,
    notation::uci::{engine::UciEngine, gui::UciGui},
};
use std::{
    collections::{VecDeque, vec_deque},
    io::{Result, Write, stdout},
    path::Path,
    process::{ExitCode, Output, exit},
    time::Duration,
};
use tokio::{io::AsyncWriteExt, select, time::sleep};
use tokio_stream::{Stream, StreamExt};

use crate::{
    analyze::ReviewGame,
    faceoff::Faceoff,
    new_profile::{NewBot, NewCommand, ProfileCommand},
};

mod analyze;
mod faceoff;
mod move_select;
mod new_profile;
mod widgets;

pub trait Runnable {
    async fn run(self) -> tokio::io::Result<()>;
}

#[derive(Parser)]
pub struct Command {
    #[clap(subcommand)]
    subcommand: SubCommand,
}

impl Runnable for Command {
    async fn run(self) -> tokio::io::Result<()> {
        match self.subcommand {
            SubCommand::New(np) => np.run().await,
            SubCommand::Fight(faceoff) => faceoff.run().await,
            SubCommand::Review(analyze_game) => analyze_game.run().await,
        }
    }
}

#[derive(Subcommand)]
pub enum SubCommand {
    /// Creates a new profile and writes to STDOUT
    New(NewCommand),
    /// Faces two chessbots off against each other
    Fight(Faceoff),
    /// Review a game from a PGN file
    Review(ReviewGame),
}

#[tokio::main]
pub async fn main() -> tokio::io::Result<()> {
    let command = Command::parse();
    command.run().await
}

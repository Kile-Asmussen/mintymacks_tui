use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    process::{ExitCode, exit},
    time::Duration,
};

use clap::{Parser, Subcommand};
use indexmap::IndexMap;
use mintymacks::notation::uci::{
    engine::{EngineOption, OptionType, SpinType, UciEngine},
    gui::UciGui,
};
use serde::{Deserialize, Serialize, de::Visitor};
use tokio::{
    fs::File,
    io::{
        AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter, stderr,
        stdin, stdout,
    },
    select,
    time::sleep,
};

use crate::{
    Runnable,
    engine::{EngineDetails, EngineHandle},
    player::EnginePlayer,
    profile::{EngineMetadata, PlayerMetadata, PlayerProfile, Profile},
};

#[derive(Parser)]
pub struct NewCommand {
    #[clap(subcommand)]
    profile: ProfileCommand,
}

impl Runnable for NewCommand {
    fn tui(&self) -> bool {
        match &self.profile {
            ProfileCommand::Player(new_player) => new_player.tui(),
            ProfileCommand::Bot(new_bot) => new_bot.tui(),
        }
    }

    async fn run(self) -> tokio::io::Result<ExitCode> {
        match self.profile {
            ProfileCommand::Player(new_player) => new_player.run().await,
            ProfileCommand::Bot(new_bot) => new_bot.run().await,
        }
    }
}

impl NewCommand {
    pub async fn create_profile(name: &str, res: String) -> tokio::io::Result<ExitCode> {
        let filename = name.to_lowercase().replace(" ", "-") + ".toml";

        if let Ok(file) = File::create_new(&filename).await {
            let mut file = BufWriter::new(file);

            file.write_all(&res[..].as_bytes()).await?;
            file.flush().await?;

            stderr()
                .write_all(format!("Profile created: {filename}\n").as_bytes())
                .await?;
            stderr().flush().await?;

            Ok(ExitCode::SUCCESS)
        } else {
            stderr()
                .write_all(format!("Profile already exists: {filename}\n").as_bytes())
                .await?;
            stderr().flush().await?;

            Ok(ExitCode::FAILURE)
        }
    }
}

#[derive(Subcommand)]
pub enum ProfileCommand {
    /// Create a new player profile
    Player(NewPlayer),

    /// Create a new bot profile
    Bot(NewBot),
}

#[derive(Parser)]
pub struct NewPlayer;

impl Runnable for NewPlayer {
    fn tui(&self) -> bool {
        false
    }

    async fn run(self) -> tokio::io::Result<ExitCode> {
        let mut cin = BufReader::new(stdin());

        stderr().write_all(b"Name: ").await?;
        stderr().flush().await?;
        let mut name = String::new();
        cin.read_line(&mut name).await?;
        name = name.trim().to_string();

        stderr().write_all(b"FIDE Title (CM, FM, IM, GM): ").await?;
        stderr().flush().await?;
        let mut title = String::new();
        cin.read_line(&mut title).await?;
        title = title.trim().to_string();

        match &title[..] {
            "CM" | "FM" | "IM" | "GM" => title.clear(),
            _ => title.clear(),
        }

        stderr().write_all(b"FIDE ELO: ").await?;
        stderr().flush().await?;
        let mut elo = String::new();
        cin.read_line(&mut elo).await?;
        elo = elo.trim().to_string();

        let elo = i32::from_str_radix(&elo, 10).unwrap_or(0);
        let res = toml::to_string(&Profile::Player(PlayerProfile {
            human: PlayerMetadata {
                name: name.clone(),
                title,
                elo,
            },
        }))
        .expect("Unable to render TOML");

        NewCommand::create_profile(&name, res).await
    }
}

#[derive(Parser)]
pub struct NewBot {
    /// Path to the bot executable
    pub bot: PathBuf,

    /// Additional arguments
    #[clap(last = true)]
    pub args: Vec<String>,
}

impl Runnable for NewBot {
    fn tui(&self) -> bool {
        false
    }

    async fn run(self) -> tokio::io::Result<ExitCode> {
        let mut engine = EngineHandle::open(&self.bot, &self.args, false).await?;

        let details = EngineDetails::extract(&mut engine).await?;

        sleep(Duration::from_millis(1000)).await;
        engine.quit().await?;

        let metadata = EngineMetadata {
            name: details.name.clone(),
            author: details.author.clone(),
            command: (self.bot, self.args),
            log: false,
        };

        let res = metadata.engine_profile_toml(&details.options);

        NewCommand::create_profile(&metadata.name, res).await
    }
}

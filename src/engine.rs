
use std::{env::current_dir, path::{Path, PathBuf}, process::Stdio, time::Duration};

use mintymacks::notation::uci::{engine::UciEngine, gui::UciGui, Uci};
use tokio::{fs::File, io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter, Error, ErrorKind}, process::{Child, ChildStderr, ChildStdin, ChildStdout, Command}, select};

pub struct Engine {
    egress: BufReader<ChildStdout>,
    ingress: BufWriter<ChildStdin>,
    process: Child                                          
}

pub struct EngineIngress<'a>(&'a mut BufWriter<ChildStdin>);

pub struct EngineEgress<'a>(&'a mut BufReader<ChildStdout>);

impl Engine {
    #[must_use]
    pub async fn new(engine: &Path, args: &[&str]) -> tokio::io::Result<Self> {

        let mut logfile = current_dir()?;
        let engine_filename =
            engine.file_name().ok_or(Error::from(ErrorKind::InvalidFilename))?;
        logfile.set_file_name(engine_filename);
        logfile.add_extension("log");


        let mut child = Command::new(engine).args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(File::create(logfile).await?.into_std().await)
            .kill_on_drop(true).spawn()?;
        
        let broken_pipe = || Error::from(ErrorKind::BrokenPipe);

        let res = Engine {
            egress: BufReader::new(child.stdout.take().ok_or_else( broken_pipe)?),
            ingress: BufWriter::new(child.stdin.take().ok_or_else( broken_pipe)?),
            process: child,
        };

        Ok(res)
    }

    #[must_use]
    pub async fn quit(&mut self) -> tokio::io::Result<()> {
        select! {
            _ = self.write(Some(&UciGui::Quit())) => {}
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        };
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.ingress.shutdown().await?;
        self.process.kill().await;
        self.process.wait().await?;
        Ok(())
    }

    #[must_use]
    pub async fn read(&mut self) -> tokio::io::Result<Option<UciEngine>> {
        EngineEgress(&mut self.egress).read().await
    }

    #[must_use]
    pub async fn write(&mut self, uci: Option<&UciGui>) -> tokio::io::Result<bool> {
        EngineIngress(&mut self.ingress).write(uci).await
    }

    #[must_use]
    pub fn split<'a>(&'a mut self) -> (EngineIngress<'a>, EngineEgress<'a>) {
        (EngineIngress(&mut self.ingress), EngineEgress(&mut self.egress))
    }
}

impl<'a> EngineIngress<'a> {
    #[must_use]
    pub async fn write(&mut self, uci: Option<&UciGui>) -> tokio::io::Result<bool> {
        let Some(uci) = uci else { return Ok(false) };
        let mut res = uci.to_string();
        res.push('\n');
        self.0.write_all(res.as_bytes()).await?;
        self.0.flush().await?;
        Ok(true)
    }
}

impl<'a> EngineEgress<'a> {
    #[must_use]
    pub async fn read(&mut self) -> tokio::io::Result<Option<UciEngine>> {
        let mut buf = String::new();
        self.0.read_line(&mut buf).await?;
        Ok(UciEngine::from_str(&buf))
    }
}
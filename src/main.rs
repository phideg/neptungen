mod config;
mod filter;
mod ftp;
mod macros;
mod render;
mod sha1dir;
mod sync;
mod template;

use crate::config::Config;
use crate::sync::Synchronizer;
use anyhow::Result;
use clap::{Parser, Subcommand};
use config::LogKind;
use flexi_logger::{FileSpec, Logger, LoggerHandle, WriteMode};
use std::fs;
use std::path::{Path, PathBuf};

fn sync(path: &Path, conf: &Config, scratch: bool, overwrite: bool) -> Result<()> {
    if scratch {
        render::build(path, conf, true)?
    }
    let mut synchronizer = Synchronizer::new(conf)?;
    if overwrite {
        synchronizer.push_all_files()
    } else {
        synchronizer.execute()
    }
}

#[derive(Parser)]
struct Build {
    /// Generate output from scratch?
    #[clap(short, long)]
    clean: bool,
}

#[derive(Parser)]
struct Sync {
    /// Overwrites all remote files?
    #[clap(short, long)]
    overwrite: bool,
    /// Do scratch build before sync?
    #[clap(short, long)]
    scratch: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Print the configuration of the config.toml file
    #[clap(alias = "print-config")]
    PrintConfig,
    /// Generate the website
    Build(Build),
    /// Synchronize the website with an sftp or ftp server
    Sync(Sync),
}

#[derive(Parser)]
#[clap(about = "Simple static website generator")]
#[clap(author, version)]
struct Arguments {
    /// Specify the path to the project. By default current directory is used.
    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    project_path: Option<PathBuf>,

    #[clap(subcommand)]
    command: Command,
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();

    let path = if let Some(path) = arguments.project_path {
        fs::canonicalize(path).expect("could not determine path")
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| fs::canonicalize(".").expect("could not determine path"))
    };

    // load configuration from config.toml if present
    let conf = Config::load(path.as_path())?;

    let _logger_handle: Option<LoggerHandle> = if let Some(log_kind) = conf.logging {
        match log_kind {
            LogKind::Stdout => Some(Logger::try_with_str("warn, neptungen=info")?.start()?),
            LogKind::File => Some(
                Logger::try_with_str("warn, neptungen=info")?
                    .log_to_file(FileSpec::default().directory(path.join(".logs")))
                    .write_mode(WriteMode::BufferAndFlush)
                    .start()?,
            ),
        }
    } else {
        None
    };

    match arguments.command {
        Command::PrintConfig => {
            conf.print();
        }
        Command::Build(build_args) => render::build(path.as_path(), &conf, build_args.clean)?,
        Command::Sync(sync_args) => sync(
            path.as_path(),
            &conf,
            sync_args.scratch,
            sync_args.overwrite,
        )?,
    };

    Ok(())
}

mod test {
    #[test]
    fn verify_app() {
        use clap::IntoApp;
        super::Arguments::into_app().debug_assert()
    }
}

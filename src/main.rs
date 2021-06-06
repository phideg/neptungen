mod config;
mod filter;
mod ftp;
mod macros;
mod render;
mod sync;
mod template;

use crate::config::Config;
use crate::sync::Synchronizer;
use anyhow::Result;
use clap::{crate_version, App, Arg, SubCommand};
use std::fs;
use std::path::Path;

fn sync(path: &Path, conf: &Config, with_build: bool, overwrite: bool) -> Result<()> {
    if with_build {
        render::build(path, conf, true)?
    }
    let mut synchronizer = Synchronizer::new(conf)?;
    if overwrite {
        synchronizer.push_all_files()
    } else {
        synchronizer.execute()
    }
}

fn main() -> Result<()> {
    let matches = App::new("neptungen")
        .about("Simple website generator")
        .version(crate_version!())
        .arg(
            Arg::with_name("project_path")
                .short("p")
                .long("project_path")
                .help("Path to the root folder of the website project")
                .multiple(false)
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("print-config")
                .about("Prints the configuration of the config.toml file"),
        )
        .subcommand(SubCommand::with_name("build").about("Generate the website"))
        .arg(
            Arg::with_name("clean")
                .short("c")
                .long("clean")
                .help("Removes all contents from the output directory before building")
                .multiple(false)
                .takes_value(false),
        )
        .subcommand(
            SubCommand::with_name("sync")
                .about("Used to synchronize the website with an ftp server")
                .arg(
                    Arg::with_name("overwrite")
                        .short("o")
                        .long("overwrite")
                        .help("Overwrites all remote files")
                        .multiple(false)
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("with_scratch_build")
                        .short("n")
                        .long("with_scratch_build")
                        .help("Executes a scratch build of the project before executing the sync")
                        .multiple(false)
                        .takes_value(false),
                ),
        )
        .get_matches();
    let path = if matches.is_present("project_path") {
        fs::canonicalize(matches.value_of("project_path").unwrap_or("."))
            .expect("could not determine path")
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| fs::canonicalize(".").expect("could not determine path"))
    };

    // load configuration from config.toml if present
    let conf = Config::load(path.as_path())?;

    match matches.subcommand() {
        ("print-config", Some(_)) => {
            conf.print();
        }
        ("build", Some(matches)) => {
            render::build(path.as_path(), &conf, matches.is_present("clean"))?
        }
        ("sync", Some(matches)) => sync(
            path.as_path(),
            &conf,
            matches.is_present("with_scratch_build"),
            matches.is_present("overwrite"),
        )?,
        _ => {
            println!("{}", matches.usage());
        }
    };

    Ok(())
}

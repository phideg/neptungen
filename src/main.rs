#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate clap;
extern crate walkdir;
extern crate pulldown_cmark;
extern crate liquid;
extern crate image;
extern crate ftp;
extern crate checksums;
extern crate term_painter;
extern crate rpassword;
extern crate rayon;

mod config;
mod render;
mod template;
mod sync;
mod filter;
mod errors;

use std::fs;
use std::path::Path;
use clap::{App, Arg, SubCommand};
use config::Config;
use sync::Synchronizer;

fn build(path: &Path, conf: &Config) {
    if let Err(ref e) = render::build(path, conf) {
        errors::print_error(e);
    }
}

fn sync(path: &Path, conf: &Config, with_build: bool, overwrite: bool) {
    if with_build {
        build(path, conf);
    }
    let mut synchronizer = Synchronizer::new(conf);
    if overwrite {
        synchronizer.push_all_files();
    } else {
        synchronizer.execute();
    }
}

fn main() {
    // parse command line options
    //
    let matches = App::new("neptungen")
        .about("Simple website generator")
        .version("0.0.1")
        .arg(Arg::with_name("project_path")
            .short("p")
            .long("project_path")
            .help("Path to the root folder of the website project")
            .multiple(false)
            .takes_value(true))
        .subcommand(SubCommand::with_name("show_config")
            .about("Prints the configuration of the config.toml file"))
        .subcommand(SubCommand::with_name("build").about("Generate the website"))
        .subcommand(SubCommand::with_name("sync")
            .about("Used to synchronize the website with an ftp server")
            .arg(Arg::with_name("overwrite")
                .short("o")
                .long("overwrite")
                .help("Overwrites all remote files")
                .multiple(false)
                .takes_value(false))
            .arg(Arg::with_name("with_build")
                .short("n")
                .long("with_build")
                .help("Build the project before executing the sync")
                .multiple(false)
                .takes_value(false)))
        .get_matches();
    let path = if matches.is_present("project_path") {
        fs::canonicalize(matches.value_of("project_path").unwrap_or("."))
            .expect("could not determine path")
    } else {
        std::env::current_dir().unwrap_or(fs::canonicalize(".").expect("could not determine path"))
    };

    // load configuration from config.toml if present
    let conf = match Config::load(path.as_path()) {
        Ok(conf) => conf,
        Err(ref e) => {
            errors::print_message(e);
            Config::new(path.as_path())
        }
    };

    match matches.subcommand() {
        ("show_config", Some(_)) => {
            conf.print();
        }
        ("build", Some(_)) => {
            build(path.as_path(), &conf);
        }
        ("sync", Some(matches)) => {
            sync(path.as_path(),
                 &conf,
                 matches.is_present("with_build"),
                 matches.is_present("overwrite"))
        }
        _ => {
            println!("{}", matches.usage());
        }
    }
}
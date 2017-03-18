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

mod config;
mod render;
mod template;
mod sync;
mod filter;

use std::fs;
use std::fs::File;
use std::io::prelude::*;
use clap::{App, Arg, SubCommand};
use config::Config;
use sync::Synchronizer;

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
        .arg(Arg::with_name("show_config")
            .short("s")
            .long("show_config")
            .help("Prints the configuration of the config.toml file")
            .multiple(false)
            .takes_value(false))
        .subcommand(SubCommand::with_name("sync")
            .about("Used to synchronize the website with an ftp server")
            .arg(Arg::with_name("overwrite")
                .short("o")
                .long("overwrite")
                .help("Overwrites all remote files")
                .multiple(false)
                .takes_value(false))
            .arg(Arg::with_name("prevent_build")
                .short("n")
                .long("prevent_build")
                .help("Only syncs the generated files")
                .multiple(false)
                .takes_value(false)))
        .get_matches();
    let path = if matches.is_present("project_path") {
        fs::canonicalize(matches.value_of("project_path").unwrap_or("."))
            .expect("could not determine path")
    } else {
        std::env::current_dir().unwrap_or(fs::canonicalize(".").expect("could not determine path"))
    };
    let show_config = matches.is_present("show_config");
    let mut sync = false;
    let mut build = true;
    // let mut overwrite = false;
    if let Some(sub_matches) = matches.subcommand_matches("sync") {
        sync = true;
        build = !sub_matches.is_present("prevent_build");
        // overwrite = sub_matches.is_present("overwrite");
    }
    // load configuration from config.toml
    //
    let mut input = String::new();
    match File::open(path.join("config.toml").as_path())
        .and_then(|mut f| f.read_to_string(&mut input)) {
        Ok(_) => {
            match toml::from_str::<Config>(input.as_str()) {
                Ok(mut conf) => {
                    conf.resolve_paths(&path.as_path());
                    if show_config {
                        conf.print();
                    }
                    if build {
                        render::build(path.as_path(), &conf);
                    }
                    if sync {
                        let mut synchronizer = Synchronizer::new(&conf);
                        synchronizer.execute();
                    }
                }
                Err(error) => panic!("failed to parse config.toml: {}", error),
            }
        }
        Err(error) => panic!("failed to open config.toml: {}", error),
    }
}

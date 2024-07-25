/*
 * Copyright (c) 2024, MLC 'Strawmelonjuice' Bloeiman
 *
 * Licensed under the GNU AFFERO GENERAL PUBLIC LICENSE Version 3, see the LICENSE file for more information.
 */

use std::fs::File;
use std::path::PathBuf;
use std::{fs, process};

use actix_web::web::Data;
use actix_web::{App, HttpServer};
use colored::Colorize;
use log::error;
#[allow(unused_imports)]
use log::info;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};
use tokio::sync::Mutex;

use crate::config::CynthiaConf;
use crate::files::CynthiaCache;
use crate::tell::horizline;

mod config;
mod files;
mod publications;
mod renders;
mod requestresponse;
mod tell;

pub struct LogSets {
    pub file_loglevel: LevelFilter,
    pub term_loglevel: LevelFilter,
    pub logfile: PathBuf,
}

#[derive(Default, Debug)]
/// Server context, containing the configuration and cache. Also implements a `tell` method for easy logging.
struct ServerContext {
    config: CynthiaConf,
    cache: CynthiaCache,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!(
        " \u{21E2} cynthiaweb {}",
        args.get(1)
            .unwrap_or(&String::from(""))
            .to_ascii_lowercase()
            .as_str()
    );
    println!("{}", horizline().purple());

    println!(
        "{} - version {}\n by {}{}{} {}!",
        "CynthiaWeb".bold().bright_purple(),
        env!("CARGO_PKG_VERSION").to_string().green(),
        "Straw".bright_red(),
        "melon".green(),
        "juice".bright_yellow(),
        "Mar".magenta()
    );
    println!("{}", horizline().purple());
    match args
        .get(1)
        .unwrap_or(&String::from(""))
        .to_ascii_lowercase()
        .as_str()
    {
        "help" => {
            println!(
                "{}",
                "Cynthia - a simple site generator/server with a focus on performance and ease of use. Targeted at smaller sites and personal projects.".bright_magenta()
            );
            println!(
                "{}",
                "Usage: cynthiaweb [command]\n\nCommands:".bright_green()
            );
            println!(
                "\t{}{}",
                "help".bold().yellow(),
                ": Displays this message.".bright_green()
            );
            println!(
                "\t{}{}",
                "start".bold().yellow(),
                ": Starts the server.".bright_green()
            );
            println!("\t{} {{{}}} <{}> ({})
            Available subcommands:
                - Add:
                    Installs a new plugin as registered in the Cynthia Plugin Index. (Does not save it to the manifest file.)

                    Options:
                        - <{}>
                            Specifies the name of the plugin to install. Is required.
                        - {{{}}}
                            (Optional) Specifies the plugin version (this will not work if a plugin has a single-version channel)
                            If not specified, latest available will be used.
                - Install:
                    Installs plugins from {} using the Cynthia Plugin Index. Useful after cloning a config.",
                     "PM".bold().yellow(),"subcommand".bright_green(),"plugin name".bright_yellow(), "plugin version".bright_purple(),
                     "plugin name".bright_yellow(),
                     "plugin version".bright_purple(),

            "cynthiapluginmanifest.json".bright_green(),);
            process::exit(0);
        }
        "start" => start().await,
        _ => start().await,
    }
}
async fn start() {
    let cd = std::env::current_dir().unwrap();
    let cynthiaconfpath = cd.join("Cynthia.toml");
    if !cynthiaconfpath.exists() {
        eprintln!("Could not find cynthia-configuration at `{}`! Have you initialised a Cynthia setup here? To do so, run `{}`.",
                  cynthiaconfpath.clone().to_string_lossy().replace("\\\\?\\", "").bright_cyan(),
                  "cynthiaweb init".bright_green());
        process::exit(1);
    }
    let config: CynthiaConf = match fs::read_to_string(cynthiaconfpath.clone()) {
        Ok(g) => match toml::from_str(&g) {
            Ok(p) => p,
            Err(e) => {
                eprintln!(
                    "{}\n\nReason:\n{}",
                    format!(
                        "Could not interpret cynthia-configuration at `{}`!",
                        cynthiaconfpath
                            .clone()
                            .to_string_lossy()
                            .replace("\\\\?\\", "")
                    )
                    .bright_red(),
                    e
                );
                process::exit(1);
            }
        },
        Err(e) => {
            eprintln!(
                "{}\n\nReason:\n{}",
                format!(
                    "Could not interpret cynthia-configuration at `{}`!",
                    cynthiaconfpath
                        .clone()
                        .to_string_lossy()
                        .replace("\\\\?\\", "")
                )
                .bright_red(),
                format!("{}", e).on_red()
            );
            process::exit(1);
        }
    };
    let logsets: LogSets = {
        fn matchlogmode(o: u8) -> LevelFilter {
            match o {
                0 => LevelFilter::Off,
                1 => LevelFilter::Error,
                2 => LevelFilter::Warn,
                3 => LevelFilter::Info,
                4 => LevelFilter::Debug,
                5 => LevelFilter::Trace,
                _ => {
                    eprintln!(
                        "{} Could not set loglevel `{}`! Ranges are 0-5 (quiet to verbose)",
                        "error:".red(),
                        o
                    );
                    process::exit(1);
                }
            }
        }
        match config.clone().logs {
            None => LogSets {
                file_loglevel: LevelFilter::Info,
                term_loglevel: LevelFilter::Warn,
                logfile: cd.join("./cynthia.log"),
            },
            Some(d) => LogSets {
                file_loglevel: match d.file_loglevel {
                    Some(l) => matchlogmode(l),
                    None => LevelFilter::Info,
                },
                term_loglevel: match d.term_loglevel {
                    Some(l) => matchlogmode(l),
                    None => LevelFilter::Warn,
                },
                logfile: match d.logfile {
                    Some(s) => cd.join(s.as_str()),
                    None => cd.join("./cynthia.log"),
                },
            },
        }
    };

    CombinedLogger::init(vec![
        TermLogger::new(
            logsets.term_loglevel,
            simplelog::Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            logsets.file_loglevel,
            simplelog::Config::default(),
            File::create(&logsets.logfile).unwrap(),
        ),
    ])
    .unwrap();
    use crate::config::CynthiaConfig;
    let server_context: ServerContext = ServerContext {
        config: config.hard_clone(),
        cache: vec![],
    };
    let _ = &server_context.tell(format!(
        "Logging to {}",
        logsets
            .logfile
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .replace("\\\\?\\", "")
    ));
    let server_context_: Data<Mutex<ServerContext>> = Data::new(Mutex::new(server_context));
    use requestresponse::serve;
    let main_server =
        match HttpServer::new(move || App::new().service(serve).app_data(server_context_.clone()))
            .bind(("localhost", config.port))
        {
            Ok(o) => {
                println!("Running on http://localhost:{}", config.port);
                o
            }
            Err(s) => {
                error!(
                    "Could not bind to port {}, error message: {}",
                    config.port, s
                );
                process::exit(1);
            }
        }
        .run();
    let _ = futures::join!(main_server, close());
}
async fn close() {
    let _ = tokio::signal::ctrl_c().await;
    println!("\n\n\nBye!\n");
    println!("{}", horizline().bright_purple());
    process::exit(0);
}

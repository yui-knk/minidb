extern crate minidb;
extern crate clap;

use std::rc::Rc;
use std::sync::RwLock;

use clap::{Arg, App, SubCommand};

use minidb::oid_manager::OidManager;
use minidb::config::{Config};
use minidb::ddl::{CreateDatabaseCommand, CreateTableCommand};
use minidb::init::{InitCommand};
use minidb::spi::{Executor};
use minidb::catalog::catalog_manager::CatalogManager;

fn main() {
    let matches = App::new("minidb")
                          .arg(Arg::with_name("base_dir")
                               .long("base_dir")
                               .required(true)
                               .value_name("DIR")
                               .takes_value(true))
                          .arg(Arg::with_name("log_level")
                               .long("log_level")
                               .required(false)
                               .default_value("warn")
                               .takes_value(true))
                          .subcommand(
                              SubCommand::with_name("init"))
                          .subcommand(
                              SubCommand::with_name("create_db")
                                  .arg(Arg::with_name("dbname")
                                       .required(true)
                                       .takes_value(true)))
                          .subcommand(
                              SubCommand::with_name("create_table")
                                  .arg(Arg::with_name("dbname")
                                       .required(true)
                                       .takes_value(true))
                                  .arg(Arg::with_name("tablename")
                                       .required(true)
                                       .takes_value(true)))
                          .subcommand(
                              SubCommand::with_name("execute")
                                  .arg(Arg::with_name("query")
                                       .required(true)
                                       .takes_value(true)))
                          .get_matches();

    let base_dir = matches.value_of("base_dir").unwrap();
    let log_level = matches.value_of("log_level").unwrap();

    let level = match log_level {
      "error" => log::Level::Error,
      "warn"  => log::Level::Warn,
      "info"  => log::Level::Info,
      "debug" => log::Level::Debug,
      "trace" => log::Level::Trace,
      _ => {
        println!("'{}' is invalid. 'trace' is used as log level", log_level);
        log::Level::Trace
      }
    };

    simple_logger::init_with_level(level).unwrap();

    let config = Rc::new(Config::new(base_dir.to_string()));

    match matches.subcommand() {
        ("init", Some(_)) => {
            let init = InitCommand::new(config.clone());
            
            match init.execute() {
                Ok(_) => {},
                Err(msg) => {
                    println!("Error: '{}'", msg);
                    ::std::process::exit(1);
                }
            }
        },
        ("create_db", Some(sub_m)) => {
            let oid_manager = RwLock::new(OidManager::new(config.clone()));
            let dbname = sub_m.value_of("dbname").unwrap();
            let create_db = CreateDatabaseCommand::new(config.clone(), oid_manager);

            match create_db.execute(dbname) {
                Ok(_) => {},
                Err(msg) => {
                    println!("Error: '{}'", msg);
                    ::std::process::exit(1);
                }
            }
        },
        ("create_table", Some(sub_m)) => {
            let oid_manager = RwLock::new(OidManager::new(config.clone()));
            let dbname = sub_m.value_of("dbname").unwrap();
            let tablename = sub_m.value_of("tablename").unwrap();
            let create_table = CreateTableCommand::new(config.clone(), oid_manager);

            match create_table.execute(dbname, tablename) {
                Ok(_) => {},
                Err(msg) => {
                    println!("Error: '{}'", msg);
                    ::std::process::exit(1);
                }
            }
        },
        ("execute", Some(sub_m)) => {
            let cmrg = CatalogManager::new(config.clone());
            let query = sub_m.value_of("query").unwrap();
            let executor = Executor::new(config.clone(), &cmrg);

            match executor.execute_query(query) {
                Ok(_) => {},
                Err(msg) => {
                    println!("Error: '{}'", msg);
                    ::std::process::exit(1);
                }
            }
        },
        (command, _) => {
            println!(
                "Unknown command '{}' is given.\nSupported commands are 'init', 'create_db' and 'create_table'",
                command
            );
            ::std::process::exit(1);
        }
    }
}
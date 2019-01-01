extern crate minidb;
extern crate clap;

use clap::{Arg, App, SubCommand};
use minidb::config::{Config};
use minidb::ddl::{CreateDatabaseCommand, CreateTableCommand};
use minidb::init::{InitCommand};

fn main() {
    let matches = App::new("minidb")
                          .arg(Arg::with_name("base_dir")
                               .long("base_dir")
                               .required(true)
                               .value_name("DIR")
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
                          .get_matches();

    let base_dir = matches.value_of("base_dir").unwrap();
    let config = Config::new(base_dir.to_string());

    match matches.subcommand() {
        ("init", Some(_)) => {
            let init = InitCommand::new(config);
            
            match init.execute() {
                Ok(_) => {},
                Err(msg) => {
                    println!("Error: '{}'", msg);
                    ::std::process::exit(1);
                }
            }
        },
        ("create_db", Some(sub_m)) => {
            let dbname = sub_m.value_of("dbname").unwrap();
            let create_db = CreateDatabaseCommand::new(config);

            match create_db.execute(dbname.to_string()) {
                Ok(_) => {},
                Err(msg) => {
                    println!("Error: '{}'", msg);
                    ::std::process::exit(1);
                }
            }
        },
        ("create_table", Some(sub_m)) => {
            let dbname = sub_m.value_of("dbname").unwrap();
            let tablename = sub_m.value_of("tablename").unwrap();
            let create_table = CreateTableCommand::new(config);

            match create_table.execute(dbname.to_string(), tablename.to_string()) {
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
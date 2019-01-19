extern crate minidb;
extern crate clap;

use std::rc::Rc;
use std::sync::RwLock;

use clap::{Arg, App, SubCommand};

use minidb::oid_manager::OidManager;
use minidb::config::{Config};
use minidb::ddl::{CreateDatabaseCommand, CreateTableCommand};
use minidb::dml::{InsertIntoCommnad, SelectFromCommnad, KeyValueBuilder};
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
                          .subcommand(
                              SubCommand::with_name("insert_into")
                                  .arg(Arg::with_name("dbname")
                                       .required(true)
                                       .takes_value(true))
                                  .arg(Arg::with_name("tablename")
                                       .required(true)
                                       .takes_value(true))
                                  .arg(Arg::with_name("id")
                                       .required(true)
                                       .takes_value(true))
                                  .arg(Arg::with_name("age")
                                       .required(true)
                                       .takes_value(true)))
                          .subcommand(
                              SubCommand::with_name("select_from")
                                  .arg(Arg::with_name("dbname")
                                       .required(true)
                                       .takes_value(true))
                                  .arg(Arg::with_name("tablename")
                                       .required(true)
                                       .takes_value(true))
                                  .arg(Arg::with_name("key")
                                       .required(true)
                                       .takes_value(true))
                                  .arg(Arg::with_name("value")
                                       .required(true)
                                       .takes_value(true)))
                          .get_matches();

    let base_dir = matches.value_of("base_dir").unwrap();
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
        ("insert_into", Some(sub_m)) => {
            let dbname = sub_m.value_of("dbname").unwrap();
            let tablename = sub_m.value_of("tablename").unwrap();
            let id = sub_m.value_of("id").unwrap();
            let age = sub_m.value_of("age").unwrap();
            let mut builder = KeyValueBuilder::new();
            let insert_into = InsertIntoCommnad::new(config);
            builder.add_pair("id".to_string(), id.to_string());
            builder.add_pair("age".to_string(), age.to_string());

            match insert_into.execute(&dbname, &tablename, builder.build()) {
                Ok(_) => {},
                Err(msg) => {
                    println!("Error: '{}'", msg);
                    ::std::process::exit(1);
                }
            }
        },
        ("select_from", Some(sub_m)) => {
            let dbname = sub_m.value_of("dbname").unwrap();
            let tablename = sub_m.value_of("tablename").unwrap();
            let select_from = SelectFromCommnad::new(config.clone());
            let key = sub_m.value_of("key").unwrap();
            let value = sub_m.value_of("value").unwrap();

            match select_from.execute(&dbname, &tablename, key, value) {
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
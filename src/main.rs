extern crate minidb;
extern crate clap;

use clap::{Arg, App};
use minidb::config::{Config};
use minidb::init::{InitCommand};

fn main() {
    let matches = App::new("minidb")
                          .arg(Arg::with_name("base_dir")
                               .long("base_dir")
                               .required(true)
                               .value_name("DIR")
                               .takes_value(true))
                          .arg(Arg::with_name("COMMAND")
                               .required(true)
                               .index(1))
                          .get_matches();

    let base_dir = matches.value_of("base_dir").unwrap();
    let command = matches.value_of("COMMAND").unwrap();

    match command {
        "init" => {
            let config = Config::new(base_dir.to_string());
            let init = InitCommand::new(config);
            
            match init.execute() {
                Ok(_) => {},
                Err(msg) => {
                    println!("Error: '{}'", msg);
                    ::std::process::exit(1);
                }
            }
        },
        _ => {
            println!("Unknown command '{}' is given", command);
            ::std::process::exit(1);
        }
    }
}
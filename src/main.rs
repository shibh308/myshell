extern crate colored;
extern crate nix;

mod execute;
mod lexer;
mod parser;
mod utils;

use colored::Colorize;
use execute::ExecutionError;
use parser::Pipe;
use std::io::{stdin, stdout, Error, Write};
use std::path::PathBuf;
use std::process::exit;

fn main() {
    main_loop();
}

fn main_loop() {
    loop {
        let currenct_dir = match std::env::current_dir() {
            Ok(path) => path.display().to_string(),
            Err(_) => "???".to_string(),
        };
        let uname = (&whoami::username()).red();
        print!(
            "{}@{}:{}: ",
            (&whoami::username()).cyan(),
            (&whoami::hostname()).cyan(),
            (&currenct_dir).green(),
        );
        stdout().flush().unwrap();
        let mut input_str = String::new();
        stdin().read_line(&mut input_str).expect("Failed to read");
        let input_str = input_str.trim_end();
        let parse_result = parser::make_parse_tree_from_str(input_str);
        match parse_result {
            Ok(root) => match execute::execute(root) {
                Ok(status) => {
                    // println!("status: {}", status);
                }
                Err(err) => {
                    if let ExecutionError::Exit = err {
                        println!("exit");
                        exit(0);
                    }
                    println!("{}", utils::ErrorEnum::ExecutionError(err));
                }
            },
            Err(err) => {
                println!("myshell {}", err);
            }
        }
    }
}

extern crate colored;
extern crate nix;

mod execute;
mod lexer;
mod parser;
mod reader;
mod utils;

use colored::Colorize;
use execute::ExecutionError;
use nix::errno::Errno;
use nix::sys::signal::SigHandler;
use nix::sys::termios::Termios;
use parser::Pipe;
use reader::ReadEnum;
use std::io::{stdin, stdout, Error, Read, Write};
use std::path::PathBuf;
use std::process::exit;

fn main() {
    prepare();
    main_loop();
}

extern "C" fn sigint_handler_fn(c: i32) {}
extern "C" fn sigquit_handler_fn(c: i32) {}

fn prepare() {
    unsafe {
        use nix::sys::signal::*;
        if let Err(_) = signal(Signal::SIGINT, SigHandler::Handler(sigint_handler_fn)) {
            println!("SIGINT handler set failed");
        }
        if let Err(_) = signal(Signal::SIGQUIT, SigHandler::Handler(sigquit_handler_fn)) {
            println!("SIGQUIT handler set failed");
        }
    }
    use nix::sys::stat::{umask, Mode};
    umask(Mode::S_IWGRP | Mode::S_IWOTH);
}

fn main_loop() {
    loop {
        let currenct_dir = match std::env::current_dir() {
            Ok(path) => path.display().to_string(),
            Err(_) => "???".to_string(),
        };
        print!(
            "{}@{}:{}: ",
            (&whoami::username()).cyan(),
            (&whoami::hostname()).cyan(),
            (&currenct_dir).green(),
        );
        stdout().flush().unwrap();
        let mut reader = reader::Reader::new();
        match reader.get_enum() {
            ReadEnum::Command(input) => {
                let parse_result = parser::make_parse_tree_from_str(&input);
                match parse_result {
                    Ok(commands) => match execute::execute(commands) {
                        Ok(status) => {
                            // println!("status: {}", status);
                        }
                        Err(ExecutionError::Exit) => {
                            println2!("exit");
                            break;
                        }
                        Err(ExecutionError::StatementIsEmpty) => {}
                        Err(err) => {
                            println2!("{}", utils::ErrorEnum::ExecutionError(err));
                        }
                    },
                    Err(err) => {
                        println2!("{}", err);
                    }
                }
            }
            ReadEnum::Comp(input) => {
                break;
            }
        }
    }
}

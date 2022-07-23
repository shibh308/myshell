extern crate colored;
extern crate nix;

mod execute;
mod lexer;
mod parser;
mod utils;

use colored::Colorize;
use execute::ExecutionError;
use nix::errno::Errno;
use nix::sys::signal::SigHandler;
use parser::Pipe;
use std::io::{stdin, stdout, Error, Write};
use std::path::PathBuf;
use std::process::exit;

fn main() {
    set_signal_handler();
    main_loop();
}

extern "C" fn sigint_handler_fn(c: i32) {}
extern "C" fn sigquit_handler_fn(c: i32) {}

fn set_signal_handler() {
    use nix::sys::signal;
    unsafe {
        if let Err(_) = signal::signal(
            signal::Signal::SIGINT,
            signal::SigHandler::Handler(sigint_handler_fn),
        ) {
            println!("SIGINT handler set failed");
        }
        if let Err(_) = signal::signal(
            signal::Signal::SIGQUIT,
            signal::SigHandler::Handler(sigquit_handler_fn),
        ) {
            println!("SIGQUIT handler set failed");
        }
    }
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
                println!("myshell: {}", err);
            }
        }
    }
}

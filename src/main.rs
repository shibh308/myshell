extern crate colored;
extern crate nix;

mod complete;
mod execute;
mod lexer;
mod parser;
mod reader;
mod utils;

use execute::ExecutionError;
use nix::errno::Errno;
use nix::sys::signal::SigHandler;
use nix::sys::termios::Termios;
use parser::Pipe;
use reader::ReadEnum;
use std::io::{stdin, stdout, Error, Read, Write};
use std::path::PathBuf;
use std::process::exit;
use utils::Env;

fn main() {
    prepare();
    main_loop();
}

extern "C" fn sigint_handler_fn(_c: i32) {}
extern "C" fn sigquit_handler_fn(_c: i32) {}

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

enum ExecuteResult {
    Success(i32),
    Empty,
    Error,
    Exit,
}

fn main_loop() {
    let mut env = Env::new();
    println!("{:?}", env);
    env.write_header();
    let mut reader = reader::Reader::new();
    loop {
        match reader.get_enum(&env) {
            ReadEnum::Command(input) => {
                let parse_result = parser::make_parse_tree_from_str(&input, &env);
                match match parse_result {
                    Ok(commands) => match execute::execute(commands) {
                        Ok(status) => {
                            // println!("status: {}", status);
                            ExecuteResult::Success(status)
                        }
                        Err(ExecutionError::Exit) => {
                            println2!("exit");
                            ExecuteResult::Exit
                        }
                        Err(ExecutionError::StatementIsEmpty) => ExecuteResult::Empty,
                        Err(err) => {
                            println2!("{}", utils::ErrorEnum::ExecutionError(err));
                            ExecuteResult::Error
                        }
                    },
                    Err(err) => {
                        println2!("{}", err);
                        ExecuteResult::Error
                    }
                } {
                    ExecuteResult::Success(status) => {
                        env.push_history(input, status);
                    }
                    ExecuteResult::Error => {
                        env.push_history(input, -1);
                    }
                    ExecuteResult::Empty => {}
                    ExecuteResult::Exit => {
                        break;
                    }
                }
                reader.clear();
                env.write_header();
            }
            ReadEnum::Comp(input) => {
                /*
                let tokens = lexer::lex(s).unwrap();
                let (pos, token_str) = if tokens.is_empty() {
                    (0, "".to_string());
                } else {
                    // lex
                    let mut tokens_without_last = tokens.clone();
                    tokens_without_last.pop();
                }
                 */
                let parse_result = parser::make_parse_tree_from_str(&input, &env);
                println2!();
                println2!("{:?}", parse_result);
                println2!();
                reader.clear();
                env.write_header();
                // break;
            }
        }
    }
}

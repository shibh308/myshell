use crate::lexer::*;
use crate::parser::*;
use nix::errno::Errno;
use nix::sys::wait::{WaitPidFlag, WaitStatus};
use nix::unistd::ForkResult;
use std::convert::Infallible;
use std::env;
use std::ffi::{CStr, CString};
use thiserror::Error;

#[derive(Clone, Error, Debug)]
pub enum CdError {
    #[error("missing argument")]
    MissingArgugment,
    #[error("too many argument (expected 1, found: {0})")]
    TooManyArgument(usize),
    #[error("{0}")]
    ExecError(String),
}

#[derive(Clone, Error, Debug)]
pub enum ExecutionError {
    #[error("command not found: {0}")]
    NotFoundError(String),
    #[error("error caused in \"{0}\"")]
    ExecError(String),
    #[error("interrupted")]
    InterruptError,
    #[error("fork error ({0})")]
    ForkError(String),
    #[error("command is empty")]
    CommandIsEmpty,
    #[error("cd error")]
    CdError(CdError),
    #[error("exit")]
    Exit,
}

fn exec_and_fork(command: Command) -> Result<i32, ExecutionError> {
    match unsafe { nix::unistd::fork() } {
        Ok(ForkResult::Parent { child }) => {
            match nix::sys::wait::waitpid(child, Some(WaitPidFlag::WCONTINUED)) {
                Ok(WaitStatus::Exited(_, status)) => Ok(status),
                Ok(WaitStatus::Signaled(_, nix::sys::signal::Signal::SIGINT, _)) => {
                    Err(ExecutionError::InterruptError)
                }
                _ => {
                    let command_str = command.str.iter().fold("".to_string(), |x, y| x + " " + y);
                    Err(ExecutionError::ExecError(command_str))
                }
            }
        }
        Ok(ForkResult::Child) => unsafe {
            let cstr = CString::new(command.str[0].clone()).unwrap();
            let cstr = CStr::from_bytes_with_nul_unchecked(cstr.to_bytes_with_nul());
            let argv = command
                .str
                .iter()
                .map(|x| CString::new(x.clone()).unwrap())
                .collect::<Vec<_>>();
            match nix::unistd::execvp(cstr, &argv) {
                Ok(_) => std::process::exit(0),
                Err(_) => {
                    println!("myshell: command not found: {}", command.str[0]);
                    std::process::exit(-1)
                }
            }
        },
        Err(err) => Err(ExecutionError::ForkError(err.to_string())),
    }
}

fn exec_command(command: Command) -> Result<i32, ExecutionError> {
    assert!(!command.str.is_empty());
    if command.str[0] == "cd" {
        if command.str.len() == 1 {
            Err(ExecutionError::CdError(CdError::MissingArgugment))
        } else if command.str.len() > 2 {
            Err(ExecutionError::CdError(CdError::TooManyArgument(
                command.str.len() - 1,
            )))
        } else {
            match env::set_current_dir(&command.str[1]) {
                Ok(_) => Ok(0),
                Err(err) => Err(ExecutionError::CdError(CdError::ExecError(err.to_string()))),
            }
        }
    } else if command.str[0] == "exit" {
        Err(ExecutionError::Exit)
    } else {
        exec_and_fork(command)
    }
}

pub fn execute(root: Pipe) -> Result<i32, ExecutionError> {
    let command = root.commands.redirect.command;
    exec_command(command)
}

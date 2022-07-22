use crate::lexer::*;
use crate::parser::*;
use std::env;
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
    #[error("command is empty")]
    CommandIsEmpty,
    #[error("cd error")]
    CdError(CdError),
    #[error("exit")]
    Exit,
}

fn exec_command(command: Command) -> Result<usize, ExecutionError> {
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
        Err(ExecutionError::CommandIsEmpty)
    }
}

pub fn execute(root: Pipe) -> Result<usize, ExecutionError> {
    let command = root.commands.redirect.command;
    exec_command(command)
}

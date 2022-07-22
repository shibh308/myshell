use crate::execute::{CdError, ExecutionError};
use crate::lexer::LexError;
use crate::parser::ParseError;
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

#[derive(Clone, Debug)]
pub enum ErrorEnum {
    ParseError(ParseError),
    LexError(LexError),
    ExecutionError(ExecutionError),
}

impl Display for ErrorEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            ErrorEnum::ParseError(err) => {
                write!(f, "ParseError: {}", err.clone())
            }
            ErrorEnum::LexError(err) => {
                write!(f, "LexError: {}", err.clone())
            }
            ErrorEnum::ExecutionError(ExecutionError::CdError(err)) => {
                write!(f, "cd: {}", err.clone())
            }
            ErrorEnum::ExecutionError(err) => {
                write!(f, "ExecutionError: {}", err.clone())
            }
        }
    }
}

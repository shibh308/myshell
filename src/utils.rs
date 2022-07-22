use crate::lexer::LexError;
use crate::parser::ParseError;
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

#[derive(Clone, Debug)]
pub enum ErrorEnum {
    ParseError(ParseError),
    LexError(LexError),
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
        }
    }
}

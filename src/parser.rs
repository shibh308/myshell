use crate::lexer;
use crate::utils::ErrorEnum;
use lexer::{LexError, Operator, Token};
use thiserror::Error;

/*
   <pipe>     ::= <commands> [ | <pipe> ]?
   <commands> ::= <redirect> [ <operator> <commands> ]?
   <redirect> ::= <command> [ <str> < ]? [ > <str> ]?
   <command>  ::= [ <str> ]+
   <operator> ::= "&&" | "||"
   <str>      ::= <char>+
   <char>     ::= any character
*/

#[derive(Clone, Error, Debug)]
pub enum ParseError {
    #[error("parse is illegally finished (at token {0})")]
    ParseFinished(usize),
    #[error("command is empty (at token {0})")]
    CommandIsEmpty(usize),
    #[error("redirected multi times (at token {0}, operator \"{1}\")")]
    MultiRedirect(usize, Operator),
    #[error("token is invalid (at token {0})")]
    InvalidToken(usize),
}

#[derive(Clone, Debug)]
pub struct Command {
    pub str: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Redirect {
    pub from: Option<String>,
    pub to: Option<String>,
    pub command: Command,
}

#[derive(Clone, Debug)]
pub struct Commands {
    pub redirect: Redirect,
    pub tail: Option<(lexer::Operator, Box<Commands>)>,
}

#[derive(Clone, Debug)]
pub struct Pipe {
    pub commands: Commands,
    pub tail: Option<Box<Pipe>>,
}

pub fn make_parse_tree_from_str(s: &str) -> Result<Pipe, ErrorEnum> {
    match lexer::lex(s) {
        Ok(tokens) => {
            let mut i = 0;
            match parse_pipe(&tokens, &mut i) {
                Ok(pipe) => {
                    if i != tokens.len() {
                        Err(ErrorEnum::ParseError(ParseError::ParseFinished(i)))
                    } else {
                        Ok(pipe)
                    }
                }
                Err(err) => Err(ErrorEnum::ParseError(err)),
            }
        }
        Err(err) => Err(ErrorEnum::LexError(err)),
    }
}

fn parse_command(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<Command, ParseError> {
    let mut v = Vec::new();
    while *l < tokens.len() {
        match &tokens[*l] {
            Token::Operator(_) => {
                break;
            }
            Token::String(s) => {
                v.push(s.clone());
            }
        }
        *l += 1;
    }
    if v.is_empty() {
        Err(ParseError::CommandIsEmpty(*l))
    } else {
        Ok(Command { str: v })
    }
}

fn parse_redirect(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<Redirect, ParseError> {
    let command = parse_command(tokens, l)?;
    let mut from = None;
    let mut to = None;
    for _ in 0..2 {
        if *l + 1 < tokens.len() {
            if let Token::String(s) = &tokens[*l + 1] {
                match &tokens[*l] {
                    Token::Operator(Operator::Less) => {
                        if from.is_some() {
                            return Err(ParseError::MultiRedirect(*l, Operator::Less));
                        }
                        from = Some(s.clone());
                        *l += 2;
                        continue;
                    }
                    Token::Operator(Operator::Greater) => {
                        if to.is_some() {
                            return Err(ParseError::MultiRedirect(*l, Operator::Less));
                        }
                        to = Some(s.clone());
                        *l += 2;
                        continue;
                    }
                    _ => {
                        break;
                    }
                }
            }
            break;
        }
    }
    Ok(Redirect { from, to, command })
}

fn parse_commands(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<Commands, ParseError> {
    let redirect = parse_redirect(tokens, l)?;
    if *l == tokens.len() {
        Ok(Commands {
            redirect,
            tail: None,
        })
    } else {
        match tokens[*l] {
            Token::Operator(Operator::AndAnd) => {
                *l += 1;
                let tail = parse_commands(tokens, l)?;
                Ok(Commands {
                    redirect,
                    tail: Some((Operator::AndAnd, Box::new(tail))),
                })
            }
            Token::Operator(Operator::OrOr) => {
                *l += 1;
                let tail = parse_commands(tokens, l)?;
                Ok(Commands {
                    redirect,
                    tail: Some((Operator::OrOr, Box::new(tail))),
                })
            }
            _ => Ok(Commands {
                redirect,
                tail: None,
            }),
        }
    }
}

fn parse_pipe(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<Pipe, ParseError> {
    let commands = parse_commands(tokens, l)?;
    if *l == tokens.len() {
        Ok(Pipe {
            commands,
            tail: None,
        })
    } else {
        if let Token::Operator(Operator::Pipe) = tokens[*l] {
            *l += 1;
            let tail = parse_pipe(tokens, l)?;
            Ok(Pipe {
                commands,
                tail: Some(Box::new(tail)),
            })
        } else {
            Err(ParseError::InvalidToken(*l))
        }
    }
}

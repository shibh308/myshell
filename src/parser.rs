use crate::lexer;
use crate::utils::ErrorEnum;
use lexer::{LexError, Operator, Token};
use thiserror::Error;

/*
   <pipe>     ::= <commands> [ < <str> ]? [ <pipe2> ]? [ > <str> ]?
   <pipe2>    ::= <commands> [ | <pipe2> ]?
   <commands> ::= <command> [ <operator> <commands> ]?
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
    #[error("token is invalid (at token {0})")]
    InvalidToken(usize),
}

#[derive(Clone, Debug)]
pub struct Command {
    pub str: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Commands {
    pub command: Command,
    pub tail: Option<(lexer::Operator, Box<Commands>)>,
}

impl Command {
    pub fn to_string(&self) -> String {
        self.str.iter().fold("".to_string(), |x, y| x + " " + y)
    }
}

impl Commands {
    pub fn to_string(&self) -> String {
        match &self.tail {
            None => self.command.to_string(),
            Some((op, tail)) => self.command.to_string() + &op.to_string() + &tail.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pipe {
    pub commands: Commands,
    pub tail: Option<Box<Pipe>>,
}

#[derive(Clone, Debug)]
pub struct Root {
    pub commands: Commands,
    pub tail: Option<Pipe>,
    pub from: Option<String>,
    pub to: Option<String>,
}

pub fn make_parse_tree_from_str(s: &str) -> Result<Root, ErrorEnum> {
    match lexer::lex(s) {
        Ok(tokens) => {
            let mut i = 0;
            match parse_root(&tokens, &mut i) {
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

fn parse_commands(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<Commands, ParseError> {
    let command = parse_command(tokens, l)?;
    if *l == tokens.len() {
        Ok(Commands {
            command,
            tail: None,
        })
    } else {
        match tokens[*l] {
            Token::Operator(Operator::AndAnd) => {
                *l += 1;
                let tail = parse_commands(tokens, l)?;
                Ok(Commands {
                    command,
                    tail: Some((Operator::AndAnd, Box::new(tail))),
                })
            }
            Token::Operator(Operator::OrOr) => {
                *l += 1;
                let tail = parse_commands(tokens, l)?;
                Ok(Commands {
                    command,
                    tail: Some((Operator::OrOr, Box::new(tail))),
                })
            }
            _ => Ok(Commands {
                command,
                tail: None,
            }),
        }
    }
}

fn parse_pipe(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<Pipe, ParseError> {
    let commands = parse_commands(tokens, l)?;
    if *l < tokens.len() {
        if let Token::Operator(Operator::Pipe) = tokens[*l] {
            *l += 1;
            let tail = parse_pipe(tokens, l)?;
            return Ok(Pipe {
                commands,
                tail: Some(Box::new(tail)),
            });
        }
    }
    Ok(Pipe {
        commands,
        tail: None,
    })
}

fn parse_root(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<Root, ParseError> {
    let commands = parse_commands(tokens, l)?;
    let mut from = None;
    let mut to = None;
    if *l + 1 < tokens.len() {
        if let Token::Operator(Operator::Less) = &tokens[*l] {
            if let Token::String(s) = &tokens[*l + 1] {
                from = Some(s.clone());
                *l += 2;
            }
        }
    }
    if *l == tokens.len() {
        return Ok(Root {
            commands,
            tail: None,
            from,
            to,
        });
    }
    if *l + 2 == tokens.len() {
        if let Token::Operator(Operator::Greater) = &tokens[*l] {
            if let Token::String(s) = &tokens[*l + 1] {
                to = Some(s.clone());
                *l += 2;
                return Ok(Root {
                    commands,
                    tail: None,
                    from,
                    to,
                });
            }
        }
    }
    if let Token::Operator(Operator::Pipe) = tokens[*l] {
        *l += 1;
        let pipe = parse_pipe(tokens, l)?;
        if *l + 2 <= tokens.len() {
            if let Token::Operator(Operator::Greater) = &tokens[*l] {
                if let Token::String(s) = &tokens[*l + 1] {
                    to = Some(s.clone());
                    *l += 2;
                }
            }
        }
        Ok(Root {
            commands,
            tail: Some(pipe),
            from,
            to,
        })
    } else {
        Err(ParseError::InvalidToken(*l))
    }
}

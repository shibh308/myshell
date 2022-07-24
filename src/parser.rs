use crate::lexer;
use crate::utils::{replace_tokens, Env, ErrorEnum};
use lexer::{LexError, Operator, Token};
use thiserror::Error;

/*
   <statement> ::= <commands> [ ; <statement> ]?
   <commands>  ::= <commands2> [ & ]? | <epsilon>
   <commands2> ::= <command> [ <operator> <commands2> ]?
   <pipe>      ::= <command> [ < <str> ]? [ <pipe2> ]? [[ > <str> ] | [ 2> <str> ]]+
   <pipe2>     ::= <command> [ | <pipe2> ]?
   <command>   ::= [ <str> ]+
   <operator>  ::= "&&" | "||"
   <str>       ::= <char>+
   <char>      ::= any character
*/

#[derive(Clone, Error, Debug)]
pub enum ParseError {
    #[error("parser does not reach the end of commands (finished at token {0})")]
    ParseFinished(usize),
    #[error("command is empty (at token {0})")]
    CommandIsEmpty(usize),
    #[error("token is invalid (at token {0})")]
    InvalidToken(usize),
    #[error("redirected multi time (at token {0})")]
    MultiRedirect(usize),
}

#[derive(Clone, Debug)]
pub struct Command {
    pub str: Vec<String>,
}

impl Command {
    pub fn to_string(&self) -> String {
        self.str
            .iter()
            .skip(1)
            .fold(self.str[0].clone(), |x, y| x + " " + y)
    }
}

#[derive(Clone, Debug)]
pub struct Pipe {
    pub command: Command,
    pub tail: Option<Box<Pipe>>,
}

impl Pipe {
    pub fn to_string(&self) -> String {
        self.command.to_string() + &self.tail.as_ref().map_or("".to_string(), |x| x.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct PipeBlock {
    pub command: Command,
    pub tail: Option<Pipe>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub to_err: Option<String>,
}

impl PipeBlock {
    pub fn to_string(&self) -> String {
        self.command.to_string()
            + &self
                .from
                .as_ref()
                .map_or("".to_string(), |x| "< ".to_owned() + &x)
            + &self
                .to
                .as_ref()
                .map_or("".to_string(), |x| "< ".to_owned() + &x)
            + &self.tail.as_ref().map_or("".to_string(), |x| x.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct Commands {
    pub head: PipeBlock,
    pub tail: Option<(lexer::Operator, Box<Commands>)>,
}

impl Commands {
    pub fn to_string(&self) -> String {
        match &self.tail {
            None => self.head.to_string(),
            Some((op, tail)) => self.head.to_string() + &op.to_string() + &tail.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Statement(pub Vec<(Commands, bool)>);

impl Statement {
    pub fn to_string(&self) -> String {
        self.0.iter().fold("".to_string(), |x, (y, b)| {
            x + "; " + &y.to_string() + (if *b { "&" } else { "" })
        })
    }
}

pub fn make_parse_tree_from_str(s: &str, env: &Env) -> Result<Statement, ErrorEnum> {
    match lexer::lex(s) {
        Ok(tokens) => make_parse_tree_from_tokens(tokens, env),
        Err(err) => Err(ErrorEnum::LexError(err)),
    }
}

pub fn make_parse_tree_from_tokens(tokens: Vec<Token>, env: &Env) -> Result<Statement, ErrorEnum> {
    let tokens = replace_tokens(tokens, env);
    let mut i = 0;
    match parse_statement(&tokens, &mut i) {
        Ok(stmt) => {
            if i != tokens.len() {
                Err(ErrorEnum::ParseError(ParseError::ParseFinished(i)))
            } else {
                Ok(stmt)
            }
        }
        Err(err) => Err(ErrorEnum::ParseError(err)),
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

fn parse_pipe(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<Pipe, ParseError> {
    let command = parse_command(tokens, l)?;
    if *l < tokens.len() {
        if let Token::Operator(Operator::Pipe) = tokens[*l] {
            *l += 1;
            let tail = parse_pipe(tokens, l)?;
            return Ok(Pipe {
                command,
                tail: Some(Box::new(tail)),
            });
        }
    }
    Ok(Pipe {
        command,
        tail: None,
    })
}

fn parse_redirection(
    tokens: &Vec<lexer::Token>,
    l: &mut usize,
    to: &mut Option<String>,
    to_err: &mut Option<String>,
) -> Option<ParseError> {
    for _ in 0..2 {
        if *l + 1 < tokens.len() {
            if let Token::Operator(Operator::Greater) = &tokens[*l] {
                if let Token::String(s) = &tokens[*l + 1] {
                    if to.is_some() {
                        return Some(ParseError::MultiRedirect(*l));
                    }
                    *to = Some(s.clone());
                    *l += 2;
                }
            } else if let Token::Operator(Operator::ErrorRedirect) = &tokens[*l] {
                if let Token::String(s) = &tokens[*l + 1] {
                    if to_err.is_some() {
                        return Some(ParseError::MultiRedirect(*l));
                    }
                    *to_err = Some(s.clone());
                    *l += 2;
                }
            }
        }
    }
    None
}

fn parse_pipe_block(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<PipeBlock, ParseError> {
    let command = parse_command(tokens, l)?;
    let mut from = None;
    let mut to = None;
    let mut to_err = None;
    if *l + 1 < tokens.len() {
        if let Token::Operator(Operator::Less) = &tokens[*l] {
            if let Token::String(s) = &tokens[*l + 1] {
                from = Some(s.clone());
                *l += 2;
            }
        }
    }
    if let Some(err) = parse_redirection(tokens, l, &mut to, &mut to_err) {
        return Err(err);
    }
    if to.is_some() || to_err.is_some() {
        return Ok(PipeBlock {
            command,
            tail: None,
            from,
            to,
            to_err,
        });
    }
    if *l == tokens.len() {
        return Ok(PipeBlock {
            command,
            tail: None,
            from,
            to,
            to_err,
        });
    }
    if let Token::Operator(Operator::Pipe) = tokens[*l] {
        *l += 1;
        let pipe = parse_pipe(tokens, l)?;
        if let Some(err) = parse_redirection(tokens, l, &mut to, &mut to_err) {
            return Err(err);
        }
        Ok(PipeBlock {
            command,
            tail: Some(pipe),
            from,
            to,
            to_err,
        })
    } else {
        Ok(PipeBlock {
            command,
            tail: None,
            from,
            to,
            to_err,
        })
    }
}

fn parse_commands(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<Commands, ParseError> {
    let head = parse_pipe_block(tokens, l)?;
    if *l == tokens.len() {
        Ok(Commands { head, tail: None })
    } else {
        match tokens[*l] {
            Token::Operator(Operator::AndAnd) => {
                *l += 1;
                let tail = parse_commands(tokens, l)?;
                Ok(Commands {
                    head,
                    tail: Some((Operator::AndAnd, Box::new(tail))),
                })
            }
            Token::Operator(Operator::OrOr) => {
                *l += 1;
                let tail = parse_commands(tokens, l)?;
                Ok(Commands {
                    head,
                    tail: Some((Operator::OrOr, Box::new(tail))),
                })
            }
            _ => Ok(Commands { head, tail: None }),
        }
    }
}

fn parse_statement(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<Statement, ParseError> {
    let mut vec = Vec::new();
    while *l < tokens.len() {
        if let Token::Operator(Operator::SemiColon) = tokens[*l] {
            *l += 1;
            continue;
        }
        let commands = parse_commands(tokens, l)?;
        let background = if *l < tokens.len() {
            if let Token::Operator(Operator::And) = tokens[*l] {
                *l += 1;
                true
            } else {
                false
            }
        } else {
            false
        };
        vec.push((commands, background));
    }
    Ok(Statement(vec))
}

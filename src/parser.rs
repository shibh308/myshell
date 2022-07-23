use crate::lexer;
use crate::utils::ErrorEnum;
use lexer::{LexError, Operator, Token};
use thiserror::Error;

/*
   <statement> ::= <commands> [ ; <statement> ]?
   <commands>  ::= <commands2> [ & ]? | <epsilon>
   <commands2> ::= <command> [ <operator> <commands2> ]?
   <pipe>      ::= <command> [ < <str> ]? [ <pipe2> ]? [ > <str> ]?
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

pub fn make_parse_tree_from_str(s: &str) -> Result<Statement, ErrorEnum> {
    match lexer::lex(s) {
        Ok(tokens) => {
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

fn parse_pipe_block(tokens: &Vec<lexer::Token>, l: &mut usize) -> Result<PipeBlock, ParseError> {
    let command = parse_command(tokens, l)?;
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
    if *l + 1 < tokens.len() {
        if let Token::Operator(Operator::Greater) = &tokens[*l] {
            if let Token::String(s) = &tokens[*l + 1] {
                to = Some(s.clone());
                *l += 2;
                return Ok(PipeBlock {
                    command,
                    tail: None,
                    from,
                    to,
                });
            }
        }
    }
    if *l == tokens.len() {
        return Ok(PipeBlock {
            command,
            tail: None,
            from,
            to,
        });
    }
    if let Token::Operator(Operator::Pipe) = tokens[*l] {
        *l += 1;
        let pipe = parse_pipe(tokens, l)?;
        if *l + 1 < tokens.len() {
            if let Token::Operator(Operator::Greater) = &tokens[*l] {
                if let Token::String(s) = &tokens[*l + 1] {
                    to = Some(s.clone());
                    *l += 2;
                }
            }
        }
        Ok(PipeBlock {
            command,
            tail: Some(pipe),
            from,
            to,
        })
    } else {
        Ok(PipeBlock {
            command,
            tail: None,
            from,
            to,
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

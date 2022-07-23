use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Clone)]
pub enum Operator {
    AndAnd,
    OrOr,
    Pipe,
    Less,
    LessLess,
    Greater,
    GreaterGreater,
    SemiColon,
}

impl Operator {
    fn to_str(&self) -> &str {
        match &self {
            Operator::AndAnd => "&&",
            Operator::OrOr => "||",
            Operator::Pipe => "|",
            Operator::Less => "<",
            Operator::LessLess => "<<",
            Operator::Greater => ">",
            Operator::GreaterGreater => ">>",
            Operator::SemiColon => ";",
        }
    }
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl std::fmt::Debug for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Clone, Error, Debug)]
pub enum LexError {
    #[error("invalid ary operator (at position {0}, operator \"{1}\")")]
    AryOperator(usize, char),
}

#[derive(Clone, Debug)]
pub enum Token {
    Operator(Operator),
    String(String),
}

pub fn lex(s: &str) -> Result<Vec<Token>, LexError> {
    let mut s = s.chars().collect::<Vec<_>>();
    s.push(' ');
    let n = s.len();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut st = 0;

    const SPECIAL_CHARS: [char; 5] = ['&', '|', '<', '>', ';'];
    let is_spl = |x: char| SPECIAL_CHARS.contains(&x) || x.is_whitespace();

    while i < n {
        if is_spl(s[i]) {
            if st != i {
                tokens.push(Token::String(s[st..i].iter().collect::<String>()));
            }
            if s[i] == '&' {
                if i + 1 < n && s[i + 1] == '&' {
                    tokens.push(Token::Operator(Operator::AndAnd));
                    i += 2;
                } else {
                    return Err(LexError::AryOperator(i + 1, '&'));
                }
            } else if s[i] == '|' {
                if i + 1 < n && s[i + 1] == '|' {
                    tokens.push(Token::Operator(Operator::OrOr));
                    i += 2;
                } else {
                    tokens.push(Token::Operator(Operator::Pipe));
                    i += 1;
                }
            } else if s[i] == '<' {
                if i + 1 < n && s[i + 1] == '<' {
                    tokens.push(Token::Operator(Operator::LessLess));
                    i += 2;
                } else {
                    tokens.push(Token::Operator(Operator::Less));
                    i += 1;
                }
            } else if s[i] == '>' {
                if i + 1 < n && s[i + 1] == '>' {
                    tokens.push(Token::Operator(Operator::GreaterGreater));
                    i += 2;
                } else {
                    tokens.push(Token::Operator(Operator::Greater));
                    i += 1;
                }
            } else if s[i] == ';' {
                tokens.push(Token::Operator(Operator::SemiColon));
                i += 1;
            } else {
                // whitespace
                i += 1;
            }
            st = i;
        } else {
            i += 1;
        }
    }
    Ok(tokens)
}

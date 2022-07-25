use crate::lexer::{lex, Token};
use crate::parser::{make_parse_tree_from_tokens, ParseError};
use crate::println2;
use crate::utils::Env;
use crate::utils::ErrorEnum;
use std::fs::File;
use std::num::ParseIntError;

#[derive(Debug)]
enum CompType {
    Bin(Option<String>),
    Path(Option<String>),
    Invalid,
}

fn get_comp_type(input: &String, env: &Env) -> CompType {
    let tokens = lex(&input).unwrap();
    if tokens.is_empty() {
        return CompType::Bin(None);
    }
    let last_whitespace = input.chars().last().unwrap().is_whitespace();
    let last_token = tokens.last().cloned().unwrap();
    let last_token_str = if !last_whitespace {
        if let Token::String(s) = last_token {
            Some(s)
        } else {
            None
        }
    } else {
        None
    };
    let check_tokens = if last_token_str.is_some() {
        tokens.iter().take(tokens.len() - 1).cloned().collect()
    } else {
        tokens.clone()
    };
    let parse_result = make_parse_tree_from_tokens(check_tokens.clone(), &env);
    let res = match &parse_result {
        Ok(stmt) if stmt.last_empty => CompType::Bin(last_token_str),
        Ok(_) => CompType::Path(last_token_str),
        Err(ErrorEnum::ParseError(ParseError::CommandIsEmpty(i))) if *i == check_tokens.len() => {
            CompType::Bin(last_token_str)
        }
        Err(ErrorEnum::ParseError(ParseError::RedirectIsEmpty(i)))
            if *i == check_tokens.len() - 1 =>
        {
            CompType::Path(last_token_str)
        }
        _ => CompType::Invalid,
    };
    match res {
        CompType::Bin(s)
            if s.clone().map_or(false, |x| {
                x.starts_with('~') || x.starts_with('.') || x.starts_with('/')
            }) =>
        {
            CompType::Path(s)
        }
        _ => res,
    }
}

pub fn comp(input: String, env: &mut Env) -> Vec<String> {
    match get_comp_type(&input, env) {
        CompType::Bin(path) => {
            let path = path.unwrap_or("".to_string());
            for ch in path.chars() {
                env.path_set.search(ch);
            }
            // TODO: use iterator
            let v = env.path_set.texts[env.path_set.get_range()]
                .iter()
                .cloned()
                .collect();
            env.path_set.reset();
            v
        }
        CompType::Path(path) => {
            println2!();
            println2!("Path: {:?}", path);
            println2!();
            Vec::new()
        }
        CompType::Invalid => {
            println2!();
            Vec::new()
        }
    }
}

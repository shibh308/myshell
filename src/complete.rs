use crate::lexer::{lex, Token};
use crate::parser::{make_parse_tree_from_tokens, ParseError};
use crate::println2;
use crate::utils::Env;
use crate::utils::ErrorEnum;
use std::fs::{File, ReadDir};
use std::io::Error;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum CompType {
    Bin(Option<String>),
    Path((Option<String>, bool)),
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
        Ok(stmt) if stmt.last_empty => CompType::Bin(last_token_str.clone()),
        Ok(_) => CompType::Path((last_token_str.clone(), true)),
        Err(ErrorEnum::ParseError(ParseError::CommandIsEmpty(i))) if *i == check_tokens.len() => {
            CompType::Bin(last_token_str.clone())
        }
        Err(ErrorEnum::ParseError(ParseError::RedirectIsEmpty(i)))
            if *i == check_tokens.len() - 1 =>
        {
            CompType::Path((last_token_str.clone(), true))
        }
        _ => CompType::Invalid,
    };
    match match res {
        CompType::Bin(s)
            if s.clone().map_or(false, |x| {
                x.starts_with('~') || x.starts_with('.') || x.starts_with('/')
            }) =>
        {
            CompType::Path((s, true))
        }
        _ => res,
    } {
        CompType::Bin(bin) => CompType::Bin(bin),
        CompType::Path((path, _)) => {
            let access_idx = if last_token_str.map_or(0, |x| x.len()) == 0 {
                1
            } else {
                2
            };
            if tokens.len() >= access_idx {
                match &tokens[tokens.len() - access_idx] {
                    Token::String(s) if s == "cd" => CompType::Path((path, false)),
                    _ => CompType::Path((path, true)),
                }
            } else {
                CompType::Path((path, true))
            }
        }
        CompType::Invalid => CompType::Invalid,
    }
}

pub fn comp(input: String, env: &mut Env) -> (usize, Vec<String>) {
    match get_comp_type(&input, env) {
        CompType::Bin(path) => {
            let fin_pos = input.len() - path.clone().map_or(0, |x| x.len());
            let path = path.unwrap_or("".to_string());
            if path.is_empty() {
                return (0, Vec::new());
            }
            for ch in path.chars() {
                env.path_set.search(ch);
            }
            let v = env.path_set.get_match_texts();
            env.path_set.reset();
            (fin_pos, v)
        }
        CompType::Path(path) => {
            let fin_pos = input.len() - path.clone().0.map_or(0, |x| x.len());
            if path.0.is_some() && path.clone().0.unwrap() == "~" {
                return (0, Vec::new());
            }
            if path.0.is_some()
                && path.clone().0.unwrap().starts_with("~")
                && !path.clone().0.unwrap().starts_with("~/")
            {
                return (0, Vec::new());
            }
            let file_ok = path.1;
            let (path, ofs_minus) = match path.0 {
                None => ("./".to_string(), 2),
                Some(path) => match path.strip_prefix("~") {
                    Some(path) => (
                        env.home_dir.display().to_string() + path,
                        env.home_dir.display().to_string().len() - 1,
                    ),
                    None if path.starts_with("/") || path.starts_with(".") => (path, 0),
                    None => ("./".to_string() + &path, 2),
                },
            };
            let pos = path.rfind('/').map(|x| x + 1).unwrap_or(0);
            let (path_parent, query) = match path.rfind('/') {
                Some(idx) => (
                    path.clone()[..idx + 1].to_string(),
                    path.clone()[idx + 1..].to_string(),
                ),
                None => ("".to_string(), path.clone()),
            };
            let files = match std::fs::read_dir(path_parent) {
                Ok(res) => res
                    .filter_map(|x| {
                        if let Ok(x) = x {
                            if let Ok(meta) = x.metadata() {
                                if let Ok(accessed) = meta.accessed() {
                                    if let Some(s) = x.file_name().to_str() {
                                        if !file_ok && !meta.is_dir() {
                                            return None;
                                        }
                                        return Some((
                                            s.to_string() + if meta.is_dir() { "/" } else { "" },
                                            accessed,
                                        ));
                                    }
                                }
                            }
                        }
                        None
                    })
                    .collect(),
                Err(_) => Vec::new(),
            };
            let mut matches = files
                .iter()
                .cloned()
                .filter(|x| {
                    (query.starts_with(".") || !x.0.starts_with(".")) && x.0.starts_with(&query)
                })
                .collect::<Vec<_>>();
            matches.sort_by(|x, y| {
                x.0.starts_with(".")
                    .cmp(&y.0.starts_with("."))
                    .reverse()
                    .then(x.1.cmp(&y.1))
                    .reverse()
            });
            let matches = matches.iter().map(|(x, y)| x).cloned().collect();
            (fin_pos + pos - ofs_minus, matches)
        }
        CompType::Invalid => (0, Vec::new()),
    }
}

use crate::execute::{CdError, ExecutionError};
use crate::lexer::LexError;
use crate::lexer::Token;
use crate::parser::ParseError;
use crate::search::Trie;
use std::fmt::{Debug, Display, Formatter};
use std::fs::{create_dir, File};
use std::io::{BufRead, BufReader};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use thiserror::Error;

#[macro_export]
macro_rules! println2 {
    () => (print!("\n\r"));
    ($($arg:tt)*) => ({
        print!($($arg)*);
        print!("\n\r");
    })
}
pub(crate) use println2;
use std::collections::BTreeSet;
use std::env::VarError;

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
            ErrorEnum::ExecutionError(ExecutionError::InterruptError) => {
                write!(f, "interrupted")
            }
            ErrorEnum::ExecutionError(ExecutionError::QuitError) => {
                write!(f, "quited")
            }
            ErrorEnum::ExecutionError(err) => {
                write!(f, "ExecutionError: {}", err.clone())
            }
        }
    }
}

fn get_history(history_file: &Option<File>) -> Vec<(i32, String)> {
    match &history_file {
        Some(file) => {
            let reader = BufReader::new(file);
            reader
                .lines()
                .filter_map(|x| x.ok())
                .filter_map(|x| match x.find(' ') {
                    Some(idx) => match &x[..idx].parse::<i32>() {
                        Ok(status) => Some((status.clone(), x[idx + 1..].to_string())),
                        Err(_) => None,
                    },
                    None => None,
                })
                .collect::<Vec<_>>()
        }
        None => Vec::new(),
    }
}

fn get_path() -> Vec<String> {
    let paths = match std::env::var("PATH") {
        Ok(path) => path.split(":").map(|x| x.to_string()).collect(),
        Err(err) => {
            println!("myshell: failed to load PATH ({})", err.to_string());
            Vec::new()
        }
    };
    let mut res = Vec::new();
    for path in paths {
        if let Ok(bins) = std::fs::read_dir(path) {
            res.extend(bins.filter_map(|x| {
                if let Ok(bin) = x {
                    if bin.file_type().map_or(false, |x| !x.is_dir()) {
                        if let Some(file_name) = bin.path().file_name() {
                            return Some(file_name.to_str().unwrap().to_string());
                        }
                    }
                }
                None
            }));
        }
    }
    res.push("cd".to_string());
    res.push("exit".to_string());
    res
}

pub struct Env {
    pub user_name: String,
    pub host_name: String,
    pub home_dir: PathBuf,
    pub path_set: Trie,
    pub history: Vec<(i32, String)>,
    pub config_dir: PathBuf,
    pub history_file: Option<File>,
    pub auto_exec_path: PathBuf,
}

impl Env {
    pub fn new() -> Env {
        const CONFIG_DIR: &str = ".myshell_conf";
        const HISTORY_PATH: &str = "history.txt";
        const AUTO_EXEC_PATH: &str = "myshellrc";
        let home_dir = dirs::home_dir().unwrap();
        let config_dir = home_dir.join(CONFIG_DIR);
        let history_path = config_dir.join(HISTORY_PATH);
        let auto_exec_path = config_dir.join(AUTO_EXEC_PATH);

        create_dir(&config_dir);
        if !history_path.exists() {
            File::create(&history_path);
        }
        if !auto_exec_path.exists() {
            File::create(&auto_exec_path);
        }
        let history_file = match File::options()
            .read(true)
            .write(true)
            .append(true)
            .open(&history_path)
        {
            Ok(file) => Some(file),
            Err(err) => {
                println!(
                    "myshell: failed to load the history file ({})",
                    err.to_string()
                );
                None
            }
        };
        let history = get_history(&history_file);

        let paths = get_path();
        let path_set = Trie::new(paths, &history);

        Env {
            user_name: whoami::username(),
            host_name: whoami::hostname(),
            path_set,
            history,
            home_dir,
            config_dir,
            history_file,
            auto_exec_path,
        }
    }

    pub fn push_history(&mut self, cmd: String, status: i32) {
        self.history.push((status, cmd.clone()));
        if let Some(file) = &self.history_file {
            let mut writer = BufWriter::new(file);
            writeln!(writer, "{} {}", status, cmd.clone());
        }
        if let Some(head) = cmd.split_ascii_whitespace().next() {
            self.path_set.add_cnt(&head.to_string());
        }
    }
}

pub fn replace_tokens(tokens: Vec<Token>, env: &Env) -> Vec<Token> {
    tokens
        .iter()
        .map(|t| match t {
            Token::Operator(_) => t.clone(),
            Token::String(s) => Token::String(match s.strip_prefix("~") {
                None => s.clone(),
                Some(suff) => env.home_dir.display().to_string() + suff,
            }),
        })
        .collect()
}

use crate::utils::Env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::num::ParseIntError;

pub fn get_history(history_file: &Option<File>) -> Vec<(i32, String)> {
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

mod lexer;
mod parser;
mod utils;

use parser::Pipe;
use std::io::{stdin, stdout, Write};

fn main() {
    main_loop();
}

fn main_loop() {
    loop {
        print!("{}@{}: ", whoami::username(), whoami::hostname());
        stdout().flush().unwrap();
        let mut input_str = String::new();
        stdin().read_line(&mut input_str).expect("Failed to read");
        let input_str = input_str.trim_end();
        let parse_result = parser::make_parse_tree_from_str(input_str);
        match parse_result {
            Ok(_) => {}
            Err(err) => {
                println!("myshell {}", err);
            }
        }
    }
}

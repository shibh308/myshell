use crate::utils::Env;
use colored::Colorize;
use nix::errno::Errno;
use nix::sys::termios::{cfmakeraw, tcgetattr, tcsetattr, SetArg};
use nix::unistd::read;
use println;
use std::io::{stdout, Write};

use crate::println2;

pub enum ReadEnum {
    Command(String),
    Comp(String),
}

pub struct Display {
    cmd: Vec<char>,
    cur: usize,
    history_cur: Option<usize>,
    suggestion: Option<Vec<char>>,
}

impl Display {
    pub fn new() -> Display {
        Display {
            cmd: Vec::new(),
            cur: 0,
            history_cur: None,
            suggestion: None,
        }
    }
    pub fn clear(&mut self) {
        self.cmd = Vec::new();
        self.cur = 0;
    }
    pub fn get_enum(&mut self, env: &Env) -> ReadEnum {
        let mut attr = tcgetattr(0).unwrap();
        let bef = attr.clone();
        cfmakeraw(&mut attr);
        tcsetattr(0, SetArg::TCSANOW, &attr);
        let res = self.stdin_read(env);
        self.restore_cursor();
        tcsetattr(0, SetArg::TCSANOW, &bef).unwrap();
        stdout().flush().unwrap();
        res
    }
    fn restore_cursor(&mut self) {
        if self.cur != 0 {
            print!("\x1b[{}C", self.cur);
        }
        self.cur = 0;
        stdout().flush().unwrap();
    }
    fn set_cmd(&mut self, cmd: String) {
        print!("{}", cmd);
        stdout().flush().unwrap();
        self.cmd = cmd.chars().collect();
        self.cur = 0;
    }
    fn reset_cmd(&mut self) {
        let diff = self.cmd.len() - self.cur;
        if diff != 0 {
            print!("\x1b[{}D", diff);
        }
        print!("\x1b[J");
        stdout().flush().unwrap();
        self.cmd = Vec::new();
        self.cur = 0;
    }
    fn add_char(&mut self, ch: char) {
        let mut buf = vec![0 as char; self.cur];
        for i in 0..self.cur {
            buf[i] = self.cmd.last().unwrap().clone();
            self.cmd.pop();
        }
        print!("\x1b[J");
        self.cmd.push(ch);
        print!("{}", ch);
        stdout().flush().unwrap();
        for &ch in buf.iter().rev() {
            self.cmd.push(ch);
            print!("{}", ch);
        }
        if self.cur != 0 {
            print!("\x1b[{}D", self.cur);
        }
        stdout().flush().unwrap();
    }
    pub fn write_header(&self, env: &Env) {
        let currenct_dir = match std::env::current_dir() {
            Ok(path) => path.display().to_string(),
            Err(_) => "???".to_string(),
        };
        print!(
            "{}@{}:{}: ",
            (env.host_name).cyan(),
            (env.user_name).cyan(),
            (&currenct_dir).green(),
        );
        std::io::stdout().flush().unwrap();
    }
    pub fn stdin_read(&mut self, env: &Env) -> ReadEnum {
        const ESCAPE: char = '\x1b';
        const DEL: char = '\x7f';

        let mut unc_buf = [0; 8];
        let mut now_idx = 0;
        let mut escape_flag = 0;

        loop {
            match read(0, &mut unc_buf[now_idx..now_idx + 1]) {
                Ok(1) => {
                    now_idx += 1;
                    if let Ok(buf_str) = std::str::from_utf8(&unc_buf[..now_idx]) {
                        now_idx = 0;
                        let ch = buf_str.chars().last().unwrap();

                        match escape_flag {
                            1 => {
                                if ch == '[' {
                                    escape_flag = 2;
                                    continue;
                                } else {
                                    escape_flag = 0;
                                }
                            }
                            2 => {
                                match ch {
                                    'A' => {
                                        self.reset_cmd();
                                        self.history_cur = match self.history_cur {
                                            Some(x) if x == 0 => None,
                                            Some(x) => Some(x - 1),
                                            None => Some(env.history.len() - 1),
                                        };
                                        if let Some(idx) = self.history_cur {
                                            self.set_cmd(env.history[idx].1.clone());
                                        }
                                    }
                                    'B' => {
                                        self.reset_cmd();
                                        self.history_cur = match self.history_cur {
                                            Some(x) if x + 1 == env.history.len() => None,
                                            Some(x) => Some(x + 1),
                                            None => Some(0),
                                        };
                                        if let Some(idx) = self.history_cur {
                                            self.set_cmd(env.history[idx].1.clone());
                                        }
                                    }
                                    'C' => {
                                        if self.cur != 0 {
                                            self.cur -= 1;
                                            print!("\x1b[C");
                                            stdout().flush().unwrap();
                                        }
                                    }
                                    'D' => {
                                        if self.cur != self.cmd.len() {
                                            self.cur += 1;
                                            print!("\x1b[D");
                                            stdout().flush().unwrap();
                                        }
                                    }
                                    _ => {}
                                }
                                escape_flag = 0;
                                continue;
                            }
                            _ => {}
                        }

                        match ch {
                            '\n' | '\r' => {
                                self.restore_cursor();
                                print!("\x1b[J");
                                println2!();
                                stdout().flush().unwrap();
                                let cmd = self.cmd.iter().collect();
                                self.cmd = Vec::new();
                                self.history_cur = None;
                                return ReadEnum::Command(cmd);
                            }
                            '\t' => {
                                self.restore_cursor();
                                self.apply_suggestion();
                                // let cmd = self.cmd.iter().collect();
                                // return ReadEnum::Comp(cmd);
                            }
                            ch if ch.is_ascii_control() => match ch {
                                DEL => {
                                    if self.cmd.pop().is_some() {
                                        print!("\x1b[D\x1b[J");
                                        stdout().flush().unwrap();
                                    }
                                    let cmd = self.cmd.iter().collect();
                                    return ReadEnum::Comp(cmd);
                                }
                                ESCAPE => {
                                    escape_flag = 1;
                                }
                                _ => {}
                            },
                            ch => {
                                self.add_char(ch);
                                let cmd = self.cmd.iter().collect();
                                return ReadEnum::Comp(cmd);
                            }
                        }
                    } else {
                        continue;
                    }
                }
                _ => {
                    continue;
                }
            }
        }
        unreachable!()
    }
    pub fn apply_suggestion(&mut self) {
        if let Some(s) = &self.suggestion {
            for &c in s {
                print!("{}", c);
                self.cmd.push(c);
            }
        }
        stdout().flush().unwrap();
        self.suggestion = None;
    }
    pub fn write_comp(&mut self, input: &String, comp: Vec<String>, ofs: usize, env: &Env) {
        print!("\x1b[J");
        // set margin
        print!("\x1b[3B");
        print!("\x1b[3A");
        // clear
        print!("\x1b[J");
        // write data
        print!("\x1b[1000D");
        self.write_header(&env);
        print!("{}", input);

        // store cursor
        print!("\x1b7");

        let pattern = if comp.is_empty() { "" } else { &input[ofs..] };
        self.suggestion = if !comp.is_empty() && pattern.len() < comp[0].len() {
            let diff = comp[0].len() - pattern.len();
            let s = &comp[0][pattern.len()..];
            print!("{}", s.dimmed());
            print!("\x1b[{}D", diff);
            Some(s.chars().collect())
        } else {
            None
        };
        println2!();
        // reverse OFF
        if !comp.is_empty() {
            print!("\x1b[?7l");
            for (i, s) in comp.iter().enumerate() {
                if i != 0 {
                    print!("\t");
                }
                print!("{}", s);
            }
            print!("\x1b[?7h");
        }
        // restore cursor
        print!("\x1b8");
        stdout().flush().unwrap();
    }
}

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

pub struct Reader {
    cmd: Vec<char>,
    cur: usize,
}

impl Reader {
    pub fn new() -> Reader {
        Reader {
            cmd: Vec::new(),
            cur: 0,
        }
    }
    pub fn clear(&mut self) {
        self.cmd = Vec::new();
        self.cur = 0;
    }
    pub fn get_enum(&mut self) -> ReadEnum {
        let mut attr = tcgetattr(0).unwrap();
        let bef = attr.clone();
        cfmakeraw(&mut attr);
        tcsetattr(0, SetArg::TCSANOW, &attr);
        let res = self.stdin_read();
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
    pub fn stdin_read(&mut self) -> ReadEnum {
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
                                println2!();
                                stdout().flush().unwrap();
                                let cmd = self.cmd.iter().collect();
                                self.cmd = Vec::new();
                                return ReadEnum::Command(cmd);
                            }
                            '\t' => {
                                let cmd = self.cmd.iter().collect();
                                return ReadEnum::Comp(cmd);
                            }
                            ch if ch.is_ascii_control() => match ch {
                                DEL => {
                                    if self.cmd.pop().is_some() {
                                        print!("\x1b[D\x1b[J");
                                        stdout().flush().unwrap();
                                    }
                                }
                                ESCAPE => {
                                    escape_flag = 1;
                                }
                                _ => {}
                            },
                            ch => {
                                self.add_char(ch);
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
}

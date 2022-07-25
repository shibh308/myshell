use crate::lexer::*;
use crate::parser::*;
use crate::println2;
use crate::utils::Env;
use crate::utils::ErrorEnum;
use nix::errno::Errno;
use nix::sys::wait::{WaitPidFlag, WaitStatus};
use nix::unistd::{close, dup2, fork, pipe, read, write, ForkResult};
use std::env;
use std::ffi::{CStr, CString};
use thiserror::Error;

#[derive(Clone, Error, Debug)]
pub enum CdError {
    #[error("missing argument")]
    MissingArgugment,
    #[error("too many argument (expected 1, found: {0})")]
    TooManyArgument(usize),
    #[error("{0}")]
    ExecError(String),
}
#[derive(Clone, Error, Debug)]
pub enum HistoryError {
    #[error("too many argument (expected 0, found: {0})")]
    TooManyArgument(usize),
    #[error("{0}")]
    ExecError(String),
}

#[derive(Clone, Error, Debug)]
pub enum ExecutionError {
    #[error("invalid operator \"{0}\"")]
    InvalidOperator(String),
    #[error("failed to open a input file: {0}")]
    InputRedirectError(String),
    #[error("failed to open a output file: {0}")]
    OutputRedirectError(String),
    #[error("failed to duplicate a file descriptor: {0}")]
    DupError(String),
    #[error("failed to close a file descriptor: {0}")]
    CloseError(String),
    #[error("command not found: {0}")]
    NotFoundError(String),
    #[error("error caused while executing: {0}")]
    ExecError(String),
    #[error("error caused in \"{0}\"")]
    ExecOtherError(String),
    #[error("interrupted")]
    InterruptError,
    #[error("quited")]
    QuitError,
    #[error("fork error ({0})")]
    ForkError(String),
    #[error("pipe error ({0})")]
    PipeError(String),
    #[error("command is empty")]
    CommandIsEmpty,
    #[error("statement is empty")]
    StatementIsEmpty,
    #[error("cd error")]
    CdError(CdError),
    #[error("history error")]
    HistoryError(HistoryError),
    #[error("exit")]
    Exit,
}

fn exec_and_fork(command: Command) -> Result<i32, ExecutionError> {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            match nix::sys::wait::waitpid(child, Some(WaitPidFlag::WCONTINUED)) {
                Ok(WaitStatus::Exited(_, status)) => Ok(status),
                Ok(WaitStatus::Signaled(_, nix::sys::signal::Signal::SIGINT, _)) => {
                    Err(ExecutionError::InterruptError)
                }
                Ok(WaitStatus::Signaled(_, nix::sys::signal::Signal::SIGQUIT, _)) => {
                    Err(ExecutionError::QuitError)
                }
                Err(err) => Err(ExecutionError::ExecError(err.to_string())),
                _ => Err(ExecutionError::ExecOtherError(command.to_string())),
            }
        }
        Ok(ForkResult::Child) => unsafe {
            let cstr = CString::new(command.str[0].clone()).unwrap();
            let cstr = CStr::from_bytes_with_nul_unchecked(cstr.to_bytes_with_nul());
            let argv = command
                .str
                .iter()
                .map(|x| CString::new(x.clone()).unwrap())
                .collect::<Vec<_>>();
            match nix::unistd::execvp(cstr, &argv) {
                Ok(_) => std::process::exit(0),
                Err(_) => {
                    println2!("myshell: command not found: {}", command.str[0]);
                    std::process::exit(-1)
                }
            }
        },
        Err(err) => Err(ExecutionError::ForkError(err.to_string())),
    }
}

fn exec_cd(command: Command) -> Result<i32, ExecutionError> {
    if command.str.len() == 1 {
        Err(ExecutionError::CdError(CdError::MissingArgugment))
    } else if command.str.len() > 2 {
        Err(ExecutionError::CdError(CdError::TooManyArgument(
            command.str.len() - 1,
        )))
    } else {
        match env::set_current_dir(&command.str[1]) {
            Ok(_) => Ok(0),
            Err(err) => Err(ExecutionError::CdError(CdError::ExecError(err.to_string()))),
        }
    }
}

fn exec_history(command: Command, env: &Env) -> Result<i32, ExecutionError> {
    if command.str.len() >= 2 {
        Err(ExecutionError::HistoryError(HistoryError::TooManyArgument(
            command.str.len() - 1,
        )))
    } else {
        for (i, (status, cmd)) in env.history.iter().enumerate() {
            println!("[{:3}][{:3}]\t{}", i, status, cmd);
        }
        Ok(0)
    }
}

fn exec_command_internal(command: Command, env: &Env) -> Result<i32, ExecutionError> {
    assert!(!command.str.is_empty());
    if command.str[0] == "cd" {
        exec_cd(command)
    } else if command.str[0] == "history" {
        exec_history(command, env)
    } else {
        exec_and_fork(command)
    }
}

fn exec_command(
    command: Command,
    input_fd: i32,
    output_fd: i32,
    err_fd: i32,
    is_tail: bool,
    env: &Env,
) -> Result<Option<i32>, ExecutionError> {
    if command.str[0] == "exit" {
        return Err(ExecutionError::Exit);
    }
    if is_tail && command.str[0] == "cd" {
        return Ok(Some(exec_cd(command)?));
    }
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            if is_tail {
                match nix::sys::wait::waitpid(child, Some(WaitPidFlag::WCONTINUED)) {
                    Ok(WaitStatus::Exited(_, status)) => Ok(Some(status)),
                    Ok(WaitStatus::Signaled(_, nix::sys::signal::Signal::SIGINT, _)) => {
                        Err(ExecutionError::InterruptError)
                    }
                    Ok(WaitStatus::Signaled(_, nix::sys::signal::Signal::SIGQUIT, _)) => {
                        Err(ExecutionError::QuitError)
                    }
                    Err(err) => Err(ExecutionError::ExecError(err.to_string())),
                    _ => Err(ExecutionError::ExecOtherError(command.to_string())),
                }
            } else {
                if input_fd != 0 {
                    if let Err(err) = close(input_fd) {
                        return Err(ExecutionError::CloseError(err.to_string()));
                    }
                }
                if output_fd != 1 {
                    if let Err(err) = close(output_fd) {
                        return Err(ExecutionError::CloseError(err.to_string()));
                    }
                }
                if err_fd != 2 {
                    if let Err(err) = close(err_fd) {
                        return Err(ExecutionError::CloseError(err.to_string()));
                    }
                }
                Ok(None)
            }
        }
        Ok(ForkResult::Child) => {
            if input_fd != 0 {
                dup2(input_fd, 0).unwrap();
            }
            if output_fd != 1 {
                dup2(output_fd, 1).unwrap();
            }
            if err_fd != 2 {
                dup2(err_fd, 2).unwrap();
            }
            if input_fd != 0 {
                close(input_fd).unwrap();
            }
            if output_fd != 1 {
                close(output_fd).unwrap();
            }
            if err_fd != 2 {
                close(err_fd).unwrap();
            }
            match exec_command_internal(command, &env) {
                Ok(status) => {
                    std::process::exit(status);
                }
                Err(err) => {
                    println2!("{}", ErrorEnum::ExecutionError(err));
                    std::process::exit(-1);
                }
            }
        }
        Err(err) => Err(ExecutionError::ForkError(err.to_string())),
    }
}

fn execute_pipe_block(pipe_block: PipeBlock, env: &Env) -> Result<i32, ExecutionError> {
    let mut input_fd = 0;
    if let Some(path) = pipe_block.from {
        let cstr = CString::new(path.clone()).unwrap();
        let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(cstr.to_bytes_with_nul()) };
        match nix::fcntl::open(
            cstr,
            nix::fcntl::OFlag::O_RDONLY,
            nix::sys::stat::Mode::all(),
        ) {
            Ok(fd) => input_fd = fd,
            Err(err) => {
                return Err(ExecutionError::InputRedirectError(err.to_string()));
            }
        }
    }
    let mut output_fd = 1;
    if let Some(path) = pipe_block.to {
        let cstr = CString::new(path.clone()).unwrap();
        let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(cstr.to_bytes_with_nul()) };
        use nix::sys::stat::Mode;
        match nix::fcntl::open(
            cstr,
            nix::fcntl::OFlag::O_WRONLY | nix::fcntl::OFlag::O_CREAT,
            Mode::S_IRUSR
                | Mode::S_IWUSR
                | Mode::S_IRGRP
                | Mode::S_IWGRP
                | Mode::S_IROTH
                | Mode::S_IWOTH,
        ) {
            Ok(fd) => output_fd = fd,
            Err(err) => {
                return Err(ExecutionError::OutputRedirectError(err.to_string()));
            }
        }
    }
    let mut err_fd = 2;
    if let Some(path) = pipe_block.to_err {
        let cstr = CString::new(path.clone()).unwrap();
        let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(cstr.to_bytes_with_nul()) };
        use nix::sys::stat::Mode;
        match nix::fcntl::open(
            cstr,
            nix::fcntl::OFlag::O_WRONLY | nix::fcntl::OFlag::O_CREAT,
            Mode::S_IRUSR
                | Mode::S_IWUSR
                | Mode::S_IRGRP
                | Mode::S_IWGRP
                | Mode::S_IROTH
                | Mode::S_IWOTH,
        ) {
            Ok(fd) => err_fd = fd,
            Err(err) => {
                return Err(ExecutionError::OutputRedirectError(err.to_string()));
            }
        }
    }

    let mut command_vec = Vec::new();

    let mut now_out_fd = output_fd;
    let mut nex_in_fd = 0;
    (now_out_fd, nex_in_fd) = match pipe_block.tail {
        None => Ok((output_fd, 0)),
        Some(_) => match pipe() {
            Ok((read_pipe, write_pipe)) => Ok((write_pipe, read_pipe)),
            Err(err) => Err(ExecutionError::PipeError(err.to_string())),
        },
    }?;
    command_vec.push((pipe_block.command, input_fd, now_out_fd));

    let mut tail = pipe_block.tail;
    while tail.is_some() {
        let mut pipe_node = tail.unwrap();
        let now_in_fd = nex_in_fd;
        (now_out_fd, nex_in_fd) = match pipe_node.tail {
            None => Ok((output_fd, 0)),
            Some(_) => match pipe() {
                Ok((read_pipe, write_pipe)) => Ok((write_pipe, read_pipe)),
                Err(err) => Err(ExecutionError::PipeError(err.to_string())),
            },
        }?;
        command_vec.push((pipe_node.command, now_in_fd, now_out_fd));
        tail = pipe_node.tail.map(|x| *x);
    }

    let mut res = None;
    for (i, (command, input_fd, output_fd)) in command_vec.iter().cloned().enumerate() {
        res = exec_command(
            command,
            input_fd,
            output_fd,
            err_fd,
            i + 1 == command_vec.len(),
            env,
        )?;
    }
    Ok(res.unwrap())
}

fn execute_commands(commands: Commands, env: &Env) -> Result<i32, ExecutionError> {
    let head_result = execute_pipe_block(commands.head, env);
    let success = head_result.clone().map_or(false, |x| x == 0);
    match commands.tail {
        None => head_result,
        Some((op, tail)) => match op {
            Operator::AndAnd => {
                if success {
                    execute_commands(*tail, env)
                } else {
                    head_result
                }
            }
            Operator::OrOr => {
                if !success {
                    execute_commands(*tail, env)
                } else {
                    head_result
                }
            }
            _ => Err(ExecutionError::InvalidOperator(op.to_string())),
        },
    }
}

fn execute_commands_background(commands: Commands, env: &Env) -> Result<i32, ExecutionError> {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child: _ }) => Ok(0),
        Ok(ForkResult::Child) => {
            let pid = std::process::id();
            println2!(
                "background process launched: \"{1}\" (pid {0})",
                pid,
                commands.to_string()
            );
            print!("\r");
            match execute_commands(commands, env) {
                Ok(status) => {
                    println2!();
                    println2!("process {} finished with code {}", pid, status);
                    std::process::exit(status);
                }
                Err(err) => {
                    println2!();
                    println2!("process {} raises an error: {}", pid, err.to_string());
                    std::process::exit(-1);
                }
            }
            unreachable!()
        }
        Err(err) => Err(ExecutionError::ForkError(err.to_string())),
    }
}

pub fn execute(stmt: Statement, env: &Env) -> Result<i32, ExecutionError> {
    let mut res = None;
    if stmt.stmt.is_empty() {
        Err(ExecutionError::StatementIsEmpty)
    } else {
        for (b, background) in stmt.stmt {
            if background {
                res = Some(execute_commands_background(b, &env)?);
            } else {
                res = Some(execute_commands(b, &env)?);
            }
        }
        Ok(res.unwrap())
    }
}

use crate::lexer::*;
use crate::parser::*;
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
pub enum ExecutionError {
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
    #[error("fork error ({0})")]
    ForkError(String),
    #[error("pipe error ({0})")]
    PipeError(String),
    #[error("command is empty")]
    CommandIsEmpty,
    #[error("cd error")]
    CdError(CdError),
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
                Err(err) => Err(ExecutionError::ExecError(err.to_string())),
                _ => {
                    let command_str = command.str.iter().fold("".to_string(), |x, y| x + " " + y);
                    Err(ExecutionError::ExecOtherError(command_str))
                }
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
                    println!("myshell: command not found: {}", command.str[0]);
                    std::process::exit(-1)
                }
            }
        },
        Err(err) => Err(ExecutionError::ForkError(err.to_string())),
    }
}

fn exec_command(command: Command) -> Result<i32, ExecutionError> {
    assert!(!command.str.is_empty());
    if command.str[0] == "cd" {
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
    } else if command.str[0] == "exit" {
        Err(ExecutionError::Exit)
    } else {
        exec_and_fork(command)
    }
}

fn exec_commands_internal(commands: Commands) -> Result<i32, ExecutionError> {
    let head_result = exec_command(commands.command);
    match commands.tail {
        None => head_result,
        Some((op, tail)) => {
            // TODO: use operator and tail
            head_result
        }
    }
}

fn exec_commands(
    commands: Commands,
    input_fd: i32,
    output_fd: i32,
    is_tail: bool,
) -> Result<Option<i32>, ExecutionError> {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            if is_tail {
                match nix::sys::wait::waitpid(child, Some(WaitPidFlag::WCONTINUED)) {
                    Ok(WaitStatus::Exited(_, status)) => Ok(Some(status)),
                    Ok(WaitStatus::Signaled(_, nix::sys::signal::Signal::SIGINT, _)) => {
                        Err(ExecutionError::InterruptError)
                    }
                    Ok(WaitStatus::Signaled(_, nix::sys::signal::Signal::SIGTERM, _)) => {
                        Err(ExecutionError::Exit)
                    }
                    Err(err) => Err(ExecutionError::ExecError(err.to_string())),
                    _ => Err(ExecutionError::ExecOtherError(commands.to_string())),
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
            if input_fd != 0 {
                close(input_fd).unwrap();
            }
            if output_fd != 1 {
                close(output_fd).unwrap();
            }
            match exec_commands_internal(commands) {
                Ok(status) => {
                    std::process::exit(status);
                }
                Err(ExecutionError::Exit) => {
                    nix::sys::signal::raise(nix::sys::signal::Signal::SIGTERM).unwrap();
                    std::process::exit(-1);
                }
                Err(err) => {
                    println!("{}", ErrorEnum::ExecutionError(err));
                    std::process::exit(-1);
                }
            }
        }
        Err(err) => Err(ExecutionError::ForkError(err.to_string())),
    }
}

pub fn execute(root: Root) -> Result<i32, ExecutionError> {
    let mut input_fd = 0;
    if let Some(path) = root.from {
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
    if let Some(path) = root.to {
        let cstr = CString::new(path.clone()).unwrap();
        let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(cstr.to_bytes_with_nul()) };
        match nix::fcntl::open(
            cstr,
            nix::fcntl::OFlag::O_WRONLY | nix::fcntl::OFlag::O_CREAT,
            nix::sys::stat::Mode::all(),
        ) {
            Ok(fd) => output_fd = fd,
            Err(err) => {
                return Err(ExecutionError::OutputRedirectError(err.to_string()));
            }
        }
    }

    let mut commands_vec = Vec::new();

    let mut now_out_fd = output_fd;
    let mut nex_in_fd = 0;
    (now_out_fd, nex_in_fd) = match root.tail {
        None => Ok((output_fd, 0)),
        Some(_) => match pipe() {
            Ok((read_pipe, write_pipe)) => Ok((write_pipe, read_pipe)),
            Err(err) => Err(ExecutionError::PipeError(err.to_string())),
        },
    }?;
    commands_vec.push((root.commands, input_fd, now_out_fd));

    let mut tail = root.tail;
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
        commands_vec.push((pipe_node.commands, now_in_fd, now_out_fd));
        tail = pipe_node.tail.map(|x| *x);
    }

    let mut res = None;
    for (i, (commands, input_fd, output_fd)) in commands_vec.iter().cloned().enumerate() {
        res = exec_commands(commands, input_fd, output_fd, i + 1 == commands_vec.len())?;
    }
    Ok(res.unwrap())
}

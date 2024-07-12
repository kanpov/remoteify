use std::{
    collections::HashMap,
    process::{Output, Stdio},
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use bytes::BytesMut;
use once_cell::sync::Lazy;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};

use crate::executor::{
    LinuxExecutor, LinuxProcess, LinuxProcessConfiguration, LinuxProcessError, LinuxProcessOutput,
    LinuxProcessPartialOutput,
};

use super::NativeLinux;

static STDOUT_BUFFERS: Lazy<Arc<RwLock<HashMap<u32, BytesMut>>>> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
static STDERR_BUFFERS: Lazy<Arc<RwLock<HashMap<u32, BytesMut>>>> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub struct NativeLinuxProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    redirect_stdout: bool,
    redirect_stderr: bool,
    pid: Option<u32>,
}

#[async_trait]
impl<'a> LinuxProcess for NativeLinuxProcess {
    fn id(&self) -> Option<u32> {
        self.pid
    }

    async fn write_to_stdin(&mut self, data: &[u8]) -> Result<usize, LinuxProcessError> {
        let stdin_ref = self.stdin.as_mut().ok_or(LinuxProcessError::StdinNotPiped)?;
        stdin_ref.write(data).await.map_err(|err| LinuxProcessError::IO(err))
    }

    async fn close_stdin(&mut self) -> Result<(), LinuxProcessError> {
        match &self.stdin {
            Some(_) => {
                drop(self.stdin.take());
                Ok(())
            }
            None => Err(LinuxProcessError::StdinNotPiped),
        }
    }

    fn get_partial_output(&self) -> Result<LinuxProcessPartialOutput, LinuxProcessError> {
        let pid = self.pid.ok_or(LinuxProcessError::ProcessIdNotFound)?;
        let mut stdout: Option<Vec<u8>> = None;
        let mut stderr: Option<Vec<u8>> = None;

        if self.redirect_stdout {
            if let Some(buf) = STDOUT_BUFFERS.read().expect("Stdout rwlock was poisoned!").get(&pid) {
                stdout = Some(buf.to_vec());
            }
        }

        if self.redirect_stderr {
            if let Some(buf) = STDERR_BUFFERS.read().expect("Stderr rwlock was poisoned!").get(&pid) {
                stderr = Some(buf.to_vec());
            }
        }

        Ok(LinuxProcessPartialOutput {
            stdout,
            stderr,
            stdout_extended: HashMap::new(),
        })
    }

    async fn await_exit_with_output(mut self) -> Result<LinuxProcessOutput, LinuxProcessError> {
        let os_output = self.child.wait_with_output().await.map_err(LinuxProcessError::IO)?;
        Ok(get_process_output(
            os_output,
            self.redirect_stdout,
            self.redirect_stderr,
        ))
    }

    async fn begin_kill(&mut self) -> Result<(), LinuxProcessError> {
        self.child.start_kill().map_err(LinuxProcessError::IO)
    }

    async fn await_exit(&mut self) -> Result<Option<i64>, LinuxProcessError> {
        self.child
            .wait()
            .await
            .map(|status| status.code().map(|i| i.into()))
            .map_err(LinuxProcessError::IO)
    }
}

#[async_trait]
impl LinuxExecutor for NativeLinux {
    async fn begin_execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<Box<dyn LinuxProcess>, LinuxProcessError> {
        let mut command = create_command_from_config(process_configuration);
        let mut child = command.spawn().map_err(LinuxProcessError::IO)?;
        let stdin = child.stdin.take();
        let pid = child.id();

        if let Some(pid) = pid {
            if !process_configuration.disable_extra_reads {
                if process_configuration.redirect_stdout {
                    STDOUT_BUFFERS
                        .write()
                        .expect("Stdout rwlock was poisoned!")
                        .insert(pid, BytesMut::new());
                    queue_capturer(&mut child, false);
                }

                if process_configuration.redirect_stderr {
                    STDERR_BUFFERS
                        .write()
                        .expect("Stderr rwlock was poisoned!")
                        .insert(pid, BytesMut::new());
                    queue_capturer(&mut child, true)
                }
            }
        }

        Ok(Box::new(NativeLinuxProcess {
            child,
            stdin,
            redirect_stdout: process_configuration.redirect_stdout,
            redirect_stderr: process_configuration.redirect_stderr,
            pid,
        }))
    }

    async fn execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<LinuxProcessOutput, LinuxProcessError> {
        let mut command = create_command_from_config(process_configuration);
        let os_output = command.output().await.map_err(LinuxProcessError::IO)?;
        Ok(get_process_output(
            os_output,
            process_configuration.redirect_stdout,
            process_configuration.redirect_stderr,
        ))
    }
}

fn get_process_output(os_output: Output, redirect_stdout: bool, redirect_stderr: bool) -> LinuxProcessOutput {
    let stdout = match redirect_stdout {
        true => Some(os_output.stdout),
        false => None,
    };

    let stderr = match redirect_stderr {
        true => Some(os_output.stderr),
        false => None,
    };

    LinuxProcessOutput {
        stdout,
        stderr,
        stdout_extended: HashMap::new(),
        status_code: os_output.status.code().map(|i| i.into()),
    }
}

fn queue_capturer(child: &mut Child, is_stderr: bool) {
    let pid = child.id().expect("Child has no PID!");
    let mut stdout: Option<ChildStdout> = None;
    let mut stderr: Option<ChildStderr> = None;

    if is_stderr {
        stderr = child.stderr.take();
    } else {
        stdout = child.stdout.take();
    }

    tokio::spawn(async move {
        let mut stdout_reader: Option<Lines<BufReader<ChildStdout>>> = None;
        let mut stderr_reader: Option<Lines<BufReader<ChildStderr>>> = None;

        if is_stderr {
            stderr_reader = Some(BufReader::new(stderr.unwrap()).lines());
        } else {
            stdout_reader = Some(BufReader::new(stdout.unwrap()).lines());
        }

        loop {
            let line_result = match is_stderr {
                true => stderr_reader.as_mut().unwrap().next_line().await,
                false => stdout_reader.as_mut().unwrap().next_line().await,
            };
            let line = match line_result {
                Ok(Some(line)) => line,
                Ok(None) => break,
                Err(_) => break,
            };

            let mut write_ref = STDOUT_BUFFERS.write().expect("Stdout rwlock was poisoned!");
            match write_ref.get_mut(&pid) {
                Some(buf) => {
                    buf.extend(line.as_bytes());
                    buf.extend(b"\n");
                }
                None => break,
            };
        }

        ()
    });
}

fn create_command_from_config(process_configuration: &LinuxProcessConfiguration) -> Command {
    let mut command = Command::new(&process_configuration.program);
    command.args(&process_configuration.args);
    command.envs(&process_configuration.envs);

    if let Some(working_dir) = &process_configuration.working_dir {
        command.current_dir(working_dir);
    }

    if process_configuration.redirect_stdout {
        command.stdout(Stdio::piped());
    } else {
        command.stdout(Stdio::null());
    }

    if process_configuration.redirect_stderr {
        command.stderr(Stdio::piped());
    } else {
        command.stderr(Stdio::null());
    }

    if process_configuration.redirect_stdin {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }

    if let Some(uid) = process_configuration.user_id {
        command.uid(uid);
    }

    if let Some(gid) = process_configuration.group_id {
        command.gid(gid);
    }

    command
}

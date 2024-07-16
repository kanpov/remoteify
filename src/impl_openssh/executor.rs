use std::{
    collections::HashMap,
    ffi::OsString,
    process::Output,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use bytes::BytesMut;
use dashmap::DashMap;
use nix::sys::signal::Signal;
use once_cell::sync::Lazy;
use openssh::{Child, ChildStdin, OwningCommand, Session, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::{
    derive_ext::DeriveExt,
    executor::{
        FinishedLinuxProcessOutput, LinuxExecutor, LinuxProcess, LinuxProcessConfiguration, LinuxProcessError,
        LinuxProcessOutput, LinuxStreamType,
    },
    filesystem::{LinuxFilesystem, LinuxOpenOptions},
};

use super::OpensshLinux;

static SYNTHETIC_ID_GENERATOR: AtomicU32 = AtomicU32::new(0);
static STDOUT_BUFFERS: Lazy<Arc<DashMap<u32, BytesMut>>> = Lazy::new(|| Arc::new(DashMap::new()));
static STDERR_BUFFERS: Lazy<Arc<DashMap<u32, BytesMut>>> = Lazy::new(|| Arc::new(DashMap::new()));

struct OpensshLinuxProcess {
    child: Child<Arc<Session>>,
    stdin: Option<ChildStdin>,
    synthetic_id: u32,
    pid_option: Option<u32>,
}

#[async_trait]
impl LinuxProcess for OpensshLinuxProcess {
    fn id(&self) -> Option<u32> {
        self.pid_option
    }

    async fn write_to_stdin(&mut self, data: &[u8]) -> Result<usize, LinuxProcessError> {
        let stdin = self.stdin.as_mut().ok_or(LinuxProcessError::StdinNotPiped)?;
        stdin
            .write(data)
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))
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

    fn get_current_output(&self) -> Result<LinuxProcessOutput, LinuxProcessError> {
        Ok(get_current_output_internal(self.synthetic_id))
    }

    async fn await_exit(self: Box<Self>) -> Result<Option<i64>, LinuxProcessError> {
        self.child
            .wait()
            .await
            .map(|status| status.code().map(|i| i.into()))
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))
    }

    async fn await_exit_with_output(self: Box<Self>) -> Result<FinishedLinuxProcessOutput, LinuxProcessError> {
        let status_code: Option<i64> = self
            .child
            .wait()
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?
            .code()
            .map(|i| i.into());
        let output = get_current_output_internal(self.synthetic_id);
        Ok(FinishedLinuxProcessOutput::join(output, status_code))
    }
}

#[async_trait]
impl LinuxExecutor for OpensshLinux {
    async fn begin_execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<Box<dyn LinuxProcess>, LinuxProcessError> {
        let (mut owning_command, pid_file) = create_owning_command(&self, &process_configuration);
        let mut child = owning_command
            .spawn()
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
        let synthetic_id = SYNTHETIC_ID_GENERATOR.fetch_add(1, Ordering::Relaxed);
        let stdin = child.stdin().take();

        if process_configuration.redirect_stdout {
            spawn_capture_task(synthetic_id, LinuxStreamType::Stdout, &mut child);
        }

        if process_configuration.redirect_stderr {
            spawn_capture_task(synthetic_id, LinuxStreamType::Stderr, &mut child);
        }

        #[allow(unused)]
        let mut pid_option: Option<u32> = None;
        let pid_file_os_str = OsString::from(pid_file);
        let pid_file_os_str = pid_file_os_str.as_os_str();
        loop {
            if let Ok(mut reader) = self.open_file(pid_file_os_str, &LinuxOpenOptions::new().read()).await {
                let mut content = String::new();
                if reader.read_to_string(&mut content).await.is_ok() {
                    pid_option = content.trim_end().parse().ok();
                    if pid_option.is_some() {
                        break;
                    }
                }
            }
        }

        Ok(Box::new(OpensshLinuxProcess {
            child,
            synthetic_id,
            stdin,
            pid_option,
        }))
    }

    async fn execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<FinishedLinuxProcessOutput, LinuxProcessError> {
        let (mut owning_command, _) = create_owning_command(&self, &process_configuration);
        let output = owning_command
            .output()
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
        Ok(conv_finished_output(output))
    }

    async fn send_signal(&self, signal: Signal, process_id: u32) -> Result<(), LinuxProcessError> {
        let mut owning_command = self.session.clone().arc_shell("kill");
        owning_command
            .arg(format!("-{}", signal.as_str()))
            .arg(process_id.to_string());
        owning_command
            .status()
            .await
            // we cannot distinguish according to the status code of "kill" since openssh crate can't reliably report exit codes of commands
            .map(|_| ())
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))
    }
}

fn conv_finished_output(value: Output) -> FinishedLinuxProcessOutput {
    FinishedLinuxProcessOutput {
        stdout: value.stdout,
        stderr: value.stderr,
        stdout_extended: HashMap::new(),
        status_code: value.status.code().map(|i| i.into()),
    }
}

#[allow(unused)]
trait ArcShellExt {
    fn arc_shell<S: AsRef<str>>(self: Arc<Self>, command: S) -> OwningCommand<Arc<Self>>;
}

impl ArcShellExt for Session {
    fn arc_shell<S: AsRef<str>>(self: Arc<Self>, command: S) -> OwningCommand<Arc<Self>> {
        let mut cmd = self.arc_command("sh");
        cmd.arg("-c").arg(command.as_ref());
        cmd
    }
}

fn get_current_output_internal(synthetic_id: u32) -> LinuxProcessOutput {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    if let Some(buf) = STDOUT_BUFFERS.get(&synthetic_id) {
        stdout = buf.to_vec();
    }

    if let Some(buf) = STDERR_BUFFERS.get(&synthetic_id) {
        stderr = buf.to_vec();
    }

    LinuxProcessOutput {
        stdout,
        stderr,
        stdout_extended: HashMap::new(),
    }
}

fn create_owning_command(
    instance: &OpensshLinux,
    process_configuration: &LinuxProcessConfiguration,
) -> (OwningCommand<Arc<Session>>, String) {
    let apply_pipes = |owning_command: &mut OwningCommand<Arc<Session>>| {
        if process_configuration.redirect_stdout {
            owning_command.stdout(Stdio::piped());
        } else {
            owning_command.stdout(Stdio::null());
        }

        if process_configuration.redirect_stderr {
            owning_command.stderr(Stdio::piped());
        } else {
            owning_command.stderr(Stdio::null());
        }

        if process_configuration.redirect_stdin {
            owning_command.stdin(Stdio::piped());
        } else {
            owning_command.stdin(Stdio::null());
        }
    };

    let (command, pid_file) = process_configuration.derive_shell_command();
    let mut owning_command = instance.session.clone().arc_shell(command);
    apply_pipes(&mut owning_command);
    (owning_command, pid_file)
}

fn spawn_capture_task(synthetic_id: u32, capturer_type: LinuxStreamType, child: &mut Child<Arc<Session>>) {
    let mut stdout_reader = None;
    let mut stderr_reader = None;

    match capturer_type {
        LinuxStreamType::Stdout => {
            STDOUT_BUFFERS.insert(synthetic_id, BytesMut::new());
            stdout_reader = Some(BufReader::new(child.stdout().take().unwrap()).lines());
        }
        LinuxStreamType::Stderr => {
            STDERR_BUFFERS.insert(synthetic_id, BytesMut::new());
            stderr_reader = Some(BufReader::new(child.stderr().take().unwrap()).lines());
        }
    }

    tokio::spawn(async move {
        loop {
            let line = match match capturer_type {
                LinuxStreamType::Stdout => stdout_reader.as_mut().unwrap().next_line().await,
                LinuxStreamType::Stderr => stderr_reader.as_mut().unwrap().next_line().await,
            } {
                Ok(Some(line)) => line + "\n",
                Ok(None) => break,
                Err(_) => break,
            };

            let write_ref = match capturer_type {
                LinuxStreamType::Stdout => STDOUT_BUFFERS.as_ref(),
                LinuxStreamType::Stderr => STDERR_BUFFERS.as_ref(),
            };
            match write_ref.get_mut(&synthetic_id) {
                Some(mut buf) => buf.extend(line.as_bytes()),
                None => break,
            }
        }
    });
}

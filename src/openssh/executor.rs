use std::{
    collections::HashMap,
    process::Output,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use bytes::BytesMut;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use openssh::{Child, ChildStdin, OwningCommand, Session, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::executor::{
    FinishedLinuxProcessOutput, LinuxExecutor, LinuxProcess, LinuxProcessConfiguration, LinuxProcessError,
    LinuxProcessOutput,
};

use super::OpensshLinux;

static SYNTHETIC_ID_GENERATOR: AtomicU32 = AtomicU32::new(0);
static STDOUT_BUFFERS: Lazy<Arc<DashMap<u32, BytesMut>>> = Lazy::new(|| Arc::new(DashMap::new()));
static STDERR_BUFFERS: Lazy<Arc<DashMap<u32, BytesMut>>> = Lazy::new(|| Arc::new(DashMap::new()));

struct OpensshLinuxProcess {
    child: Child<Arc<Session>>,
    stdin: Option<ChildStdin>,
    synthetic_id: u32,
}

#[async_trait]
impl LinuxProcess for OpensshLinuxProcess {
    fn id(&self) -> Option<u32> {
        None
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

    async fn begin_kill(&mut self) -> Result<(), LinuxProcessError> {
        self.write_to_stdin(b"^C").await.map(|_| ())
    }
}

#[async_trait]
impl LinuxExecutor for OpensshLinux {
    async fn begin_execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<Box<dyn LinuxProcess>, LinuxProcessError> {
        let mut owning_command = create_owning_command(&self, &process_configuration);
        let mut child = owning_command
            .spawn()
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
        let synthetic_id = SYNTHETIC_ID_GENERATOR.fetch_add(1, Ordering::Relaxed);
        let stdin = child.stdin().take();

        if process_configuration.redirect_stdout {
            spawn_capture_task(synthetic_id, CapturerType::Stdout, &mut child);
        }

        if process_configuration.redirect_stderr {
            spawn_capture_task(synthetic_id, CapturerType::Stderr, &mut child);
        }

        Ok(Box::new(OpensshLinuxProcess {
            child,
            synthetic_id,
            stdin,
        }))
    }

    async fn execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<FinishedLinuxProcessOutput, LinuxProcessError> {
        let mut owning_command = create_owning_command(&self, &process_configuration);
        let output = owning_command
            .output()
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
        Ok(conv_finished_output(output))
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
) -> OwningCommand<Arc<Session>> {
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

    // when a working dir needs or an env var need to be set, a shell command must be used, otherwise we use a regular command
    // the session arc needs to be cloned since the command will take ownership of the session

    if process_configuration.working_dir.is_none() && process_configuration.envs.is_empty() {
        let mut owning_command = instance.session.clone().arc_command(&process_configuration.program);
        owning_command.args(&process_configuration.args);
        apply_pipes(&mut owning_command);
        owning_command
    } else {
        let mut env_str = String::new();

        for (env_key, env_value) in &process_configuration.envs {
            env_str.push_str(format!("{}={}", env_key, env_value).as_str());
            env_str.push_str(" ");
        }

        let mut command = process_configuration.program.clone();
        if process_configuration.args.len() > 0 {
            command.push_str(" ");
            for arg in &process_configuration.args {
                command.push_str(shell_escape::unix::escape(arg.into()).as_ref());
                command.push_str(" ");
            }
        }
        if process_configuration.envs.len() > 0 {
            command = env_str + command.as_str();
        }

        if let Some(working_dir) = &process_configuration.working_dir {
            command = format!("(cd {} && {})", working_dir, command);
        }

        let mut owning_command = instance.session.clone().arc_shell(command);
        apply_pipes(&mut owning_command);
        owning_command
    }
}

enum CapturerType {
    Stdout,
    Stderr,
}

fn spawn_capture_task(synthetic_id: u32, capturer_type: CapturerType, child: &mut Child<Arc<Session>>) {
    let mut stdout_reader = None;
    let mut stderr_reader = None;

    match capturer_type {
        CapturerType::Stdout => {
            STDOUT_BUFFERS.insert(synthetic_id, BytesMut::new());
            stdout_reader = Some(BufReader::new(child.stdout().take().unwrap()).lines());
        }
        CapturerType::Stderr => {
            STDERR_BUFFERS.insert(synthetic_id, BytesMut::new());
            stderr_reader = Some(BufReader::new(child.stderr().take().unwrap()).lines());
        }
    }

    tokio::spawn(async move {
        loop {
            let line = match match capturer_type {
                CapturerType::Stdout => stdout_reader.as_mut().unwrap().next_line().await,
                CapturerType::Stderr => stderr_reader.as_mut().unwrap().next_line().await,
            } {
                Ok(Some(line)) => line + "\n",
                Ok(None) => break,
                Err(_) => break,
            };

            let write_ref = match capturer_type {
                CapturerType::Stdout => STDOUT_BUFFERS.as_ref(),
                CapturerType::Stderr => STDERR_BUFFERS.as_ref(),
            };
            match write_ref.get_mut(&synthetic_id) {
                Some(mut buf) => buf.extend(line.as_bytes()),
                None => break,
            }
        }
    });
}

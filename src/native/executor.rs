use std::process::{Output, Stdio};

use async_trait::async_trait;
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin, Command},
};

use crate::executor::{LinuxExecutor, LinuxProcess, LinuxProcessConfiguration, LinuxProcessError, LinuxProcessOutput};

use super::NativeLinux;

pub struct NativeLinuxProcess {
    child: Child,
    stdin: Option<ChildStdin>,
}

#[async_trait]
impl<'a> LinuxProcess for NativeLinuxProcess {
    fn id(&self) -> Option<u32> {
        self.child.id()
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

    async fn await_exit_with_output(self) -> Result<LinuxProcessOutput, LinuxProcessError> {
        let os_output = self.child.wait_with_output().await.map_err(LinuxProcessError::IO)?;
        Ok(os_output.into())
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
    ) -> Result<NativeLinuxProcess, LinuxProcessError> {
        let mut command = create_command_from_config(process_configuration);
        let mut child = command.spawn().map_err(LinuxProcessError::IO)?;
        let stdin = child.stdin.take();

        Ok(NativeLinuxProcess { child, stdin })
    }

    async fn execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<LinuxProcessOutput, LinuxProcessError> {
        let mut command = create_command_from_config(process_configuration);
        let os_output = command.output().await.map_err(LinuxProcessError::IO)?;
        Ok(os_output.into())
    }
}

impl From<Output> for LinuxProcessOutput {
    fn from(value: Output) -> Self {
        LinuxProcessOutput {
            stdout: value.stdout,
            stderr: value.stderr,
            status_code: value.status.code().map(|i| i.into()),
        }
    }
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

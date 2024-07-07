use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct LinuxProcessConfiguration {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) envs: HashMap<String, String>,
    pub(crate) working_dir: Option<PathBuf>,
    pub(crate) redirect_stdout: bool,
    pub(crate) redirect_stderr: bool,
    pub(crate) user_id: Option<u32>,
    pub(crate) group_id: Option<u32>,
    pub(crate) process_group_id: Option<u32>,
}

impl LinuxProcessConfiguration {
    pub fn new(program: String) -> LinuxProcessConfiguration {
        LinuxProcessConfiguration {
            program,
            args: Vec::new(),
            envs: HashMap::new(),
            working_dir: None,
            redirect_stdout: false,
            redirect_stderr: false,
            user_id: None,
            group_id: None,
            process_group_id: None,
        }
    }

    pub fn arg(&mut self, argument: String) -> &mut Self {
        self.args.push(argument);
        self
    }

    pub fn args(&mut self, arguments: &mut Vec<String>) -> &mut Self {
        self.args.append(arguments);
        self
    }

    pub fn env(&mut self, key: String, value: String) -> &mut Self {
        self.envs.insert(key, value);
        self
    }

    pub fn envs(&mut self, environment: HashMap<String, String>) -> &mut Self {
        self.envs.extend(environment);
        self
    }

    pub fn clear_env(&mut self) -> &mut Self {
        self.envs.clear();
        self
    }

    pub fn working_dir(&mut self, working_dir: PathBuf) -> &mut Self {
        self.working_dir = Some(working_dir);
        self
    }

    pub fn redirect_stdout(&mut self) -> &mut Self {
        self.redirect_stdout = true;
        self
    }

    pub fn redirect_stderr(&mut self) -> &mut Self {
        self.redirect_stderr = true;
        self
    }

    pub fn user_id(&mut self, user_id: u32) -> &mut Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn group_id(&mut self, group_id: u32) -> &mut Self {
        self.group_id = Some(group_id);
        self
    }

    pub fn process_group_id(&mut self, process_group_id: u32) -> &mut Self {
        self.process_group_id = Some(process_group_id);
        self
    }
}

#[derive(Debug)]
pub enum LinuxProcessError {
    UnsupportedOperation,
    IO(std::io::Error),
    CouldNotAcquireStream,
    Other(Box<dyn std::error::Error>),
}

#[derive(Debug, Clone)]
pub struct LinuxProcessOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status_code: Option<i64>,
}

#[async_trait]
pub trait LinuxProcess {
    fn id(&self) -> Option<u32>;

    async fn await_exit(&mut self) -> Result<Option<i64>, LinuxProcessError>;

    async fn await_exit_with_output(self) -> Result<LinuxProcessOutput, LinuxProcessError>;

    async fn send_kill_request(&mut self) -> Result<(), LinuxProcessError>;
}

#[async_trait]
pub trait LinuxExecutor {
    async fn begin_execute(
        &self,
        process_configuration: LinuxProcessConfiguration,
    ) -> Result<impl LinuxProcess, LinuxProcessError>;

    async fn execute(
        &self,
        process_configuration: LinuxProcessConfiguration,
    ) -> Result<LinuxProcessOutput, LinuxProcessError>;
}

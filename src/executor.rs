use std::collections::HashMap;

use async_trait::async_trait;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxProcessConfiguration {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) envs: HashMap<String, String>,
    pub(crate) working_dir: Option<String>,
    pub(crate) redirect_stdout: bool,
    pub(crate) redirect_stdin: bool,
    pub(crate) redirect_stderr: bool,
    pub(crate) user_id: Option<u32>,
    pub(crate) group_id: Option<u32>,
    pub(crate) process_group_id: Option<u32>,
}

pub enum LinuxProcessExpectation {
    StringMatch {
        value: String,
        match_type: StringMatchType,
        case_sensitive: bool,
    },
    Regex(Regex),
    StreamClosure(StreamType),
}

pub enum StringMatchType {
    Equals,
    Contains,
    StartsWith,
    EndsWith,
}

pub enum StreamType {
    Stdout,
    Stderr,
}

impl LinuxProcessConfiguration {
    pub fn new(program: impl Into<String>) -> LinuxProcessConfiguration {
        LinuxProcessConfiguration {
            program: program.into(),
            args: Vec::new(),
            envs: HashMap::new(),
            working_dir: None,
            redirect_stdout: false,
            redirect_stdin: false,
            redirect_stderr: false,
            user_id: None,
            group_id: None,
            process_group_id: None,
        }
    }

    pub fn arg(&mut self, argument: impl Into<String>) -> &mut Self {
        self.args.push(argument.into());
        self
    }

    pub fn args(&mut self, arguments: Vec<impl Into<String>>) -> &mut Self {
        for arg in arguments {
            self.args.push(arg.into());
        }
        self
    }

    pub fn env(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.envs.insert(key.into(), value.into());
        self
    }

    pub fn envs<K: Into<String>, V: Into<String>>(&mut self, environment: HashMap<K, V>) -> &mut Self {
        for (env_key, env_value) in environment {
            self.envs.insert(env_key.into(), env_value.into());
        }
        self
    }

    pub fn clear_env(&mut self) -> &mut Self {
        self.envs.clear();
        self
    }

    pub fn working_dir(&mut self, working_dir: impl Into<String>) -> &mut Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    pub fn redirect_stdout(&mut self) -> &mut Self {
        self.redirect_stdout = true;
        self
    }

    pub fn redirect_stdin(&mut self) -> &mut Self {
        self.redirect_stdin = true;
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
    KillRequestUnsupported,
    ProcessIdNotFound,
    StdinNotPiped,
    IO(std::io::Error),
    Other(Box<dyn std::error::Error>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedLinuxProcessOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_extended: HashMap<u32, Vec<u8>>,
    pub status_code: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxProcessOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_extended: HashMap<u32, Vec<u8>>,
}

impl FinishedLinuxProcessOutput {
    pub fn join(output: LinuxProcessOutput, status_code: Option<i64>) -> FinishedLinuxProcessOutput {
        FinishedLinuxProcessOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            stdout_extended: output.stdout_extended,
            status_code,
        }
    }
}

#[async_trait]
pub trait LinuxProcess: Send {
    fn id(&self) -> Option<u32>;

    async fn write_to_stdin(&mut self, data: &[u8]) -> Result<usize, LinuxProcessError>;

    async fn close_stdin(&mut self) -> Result<(), LinuxProcessError>;

    fn get_current_output(&self) -> Result<LinuxProcessOutput, LinuxProcessError>;

    async fn await_exit(mut self: Box<Self>) -> Result<Option<i64>, LinuxProcessError>;

    async fn await_exit_with_output(mut self: Box<Self>) -> Result<FinishedLinuxProcessOutput, LinuxProcessError>;

    async fn send_kill_request(&mut self) -> Result<(), LinuxProcessError> {
        Err(LinuxProcessError::KillRequestUnsupported)
    }
}

#[async_trait]
pub trait LinuxExecutor {
    async fn begin_execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<Box<dyn LinuxProcess>, LinuxProcessError>;

    async fn execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<FinishedLinuxProcessOutput, LinuxProcessError>;
}

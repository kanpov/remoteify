use std::collections::HashMap;

use async_trait::async_trait;
use regex::Regex;
use shell_escape::unix::escape;
use uuid::Uuid;

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

    pub fn desugar_to_shell_command(&self) -> (String, String) {
        // example of desugared command:
        // (cd working_dir && echo $$ > /tmp/pid-UUID && env1=val1 env2=val2 ... exec actual_command arg1 arg2 ...)

        let pid_file = format!("/tmp/pid-{}", Uuid::new_v4());
        let mut sections: Vec<String> = Vec::new();

        // 1. working dir
        if let Some(working_dir) = &self.working_dir {
            sections.push(format!("cd {}", working_dir));
        }
        // 2. echo PID into a file to be read via SFTP later
        sections.push(format!("echo $$ > {}", pid_file));
        // 3.1. prepend with environment variables
        let mut exec_section = String::new();
        if !self.envs.is_empty() {
            for (env_key, env_value) in &self.envs {
                exec_section.push_str(env_key);
                exec_section.push('=');
                exec_section.push_str(env_value);
                exec_section.push(' ');
            }
        }
        // 3.2. run the command with exec, thus giving it the shell's PID
        exec_section.push_str("exec ");
        exec_section.push_str(&self.program);
        // 3.3. append shell-escaped args to the command
        if !self.args.is_empty() {
            exec_section.push(' ');
            for arg in &self.args {
                exec_section.push_str(escape(arg.into()).to_string().as_str());
                exec_section.push(' ');
            }
            exec_section = exec_section.trim_end().into();
        }
        sections.push(exec_section);

        // join sections with && and wrap them in a subshell
        let mut output = String::from('(');
        output.push_str(sections.join(" && ").as_str());
        output.push(')');

        (output, pid_file)
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

use shell_escape::unix::escape;
use uuid::Uuid;

use crate::executor::LinuxProcessConfiguration;

pub trait DeriveExt {
    fn derive_shell_command(&self) -> (String, String);
}

impl DeriveExt for LinuxProcessConfiguration {
    fn derive_shell_command(&self) -> (String, String) {
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

use common::{OpensshData, RusshData};
use futures::{future::BoxFuture, FutureExt};
use remoteify::{
    executor::{FinishedLinuxProcessOutput, LinuxExecutor, LinuxProcessConfiguration},
    native::NativeLinux,
};
use uuid::Uuid;

mod common;
#[tokio::test]
async fn simple_command_outputting() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/echo");
            config.arg("--help").redirect_stdout();
            let process_output = executor.execute(&config).await.unwrap();
            let stdout = String::from_utf8(process_output.stdout).unwrap();

            assert_eq!(process_output.status_code.expect("No status code provided"), 0);
            assert!(process_output.stderr.is_empty());
            assert!(stdout.contains("Full documentation <https://www.gnu.org/software/coreutils/echo>"));
        }
        .boxed()
    })
    .await;
}

#[tokio::test]
async fn simple_command_erroring() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/cat");
            let id = Uuid::new_v4();
            config.arg(format!("/tmp/{}", id));
            config.redirect_stderr();
            let process_output = executor.execute(&config).await.unwrap();
            let stderr = String::from_utf8(process_output.stderr).unwrap();

            assert_ne!(process_output.status_code.expect("No status code provided"), 0);
            assert!(stderr.contains(id.to_string().as_str()));
        }
        .boxed()
    })
    .await;
}

#[tokio::test]
async fn simple_command_accepting_env_vars() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/bash");
            config
                .redirect_stdout()
                .redirect_stderr()
                .env("ENV_KEY", "ENV_VALUE")
                .env("OTHER_KEY", "OTHER_VALUE")
                .args(vec!["-c", "printenv ENV_KEY && printenv OTHER_KEY"]);
            let process_output = executor.execute(&config).await.unwrap();
            assert_eq!(process_output.status_code, Some(0));
            assert!(process_output.stderr.is_empty());
            assert_eq!(
                String::from_utf8(process_output.stdout).unwrap(),
                "ENV_VALUE\nOTHER_VALUE\n"
            );
        }
        .boxed()
    })
    .await;
}

#[tokio::test]
async fn simple_command_inside_working_dir() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/pwd");
            config.redirect_stdout().redirect_stderr().working_dir("/tmp");
            let process_output = executor.execute(&config).await.unwrap();
            assert_ok_execution(process_output, "/tmp\n");
        }
        .boxed()
    })
    .await;
}

#[tokio::test]
async fn simple_command_with_various_options() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/bash");
            config
                .working_dir("/usr")
                .redirect_stdout()
                .redirect_stderr()
                .env("ENV1", "VAL1")
                .env("ENV2", "VAL2")
                .args(vec![
                    "-c",
                    "echo stdout && pwd && printenv ENV1 && printenv ENV2 && echo stderr >&2",
                ]);
            let process_output = executor.execute(&config).await.unwrap();
            let status_code = process_output.status_code.unwrap();
            // the correct status is 127, however, reporting a correct status code in a ssh context is difficult
            // and not fixable on the side of this library
            assert!(status_code == 0 || status_code == 127);
            assert_eq!(
                String::from_utf8(process_output.stdout).unwrap(),
                "stdout\n/usr\nVAL1\nVAL2\n"
            );
            assert_eq!(String::from_utf8(process_output.stderr).unwrap(), "stderr\n");
        }
        .boxed()
    })
    .await;
}

#[tokio::test]
async fn interactive_command_with_immediate_eof_returning_only_status() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/bash");
            config.redirect_stdin();
            let mut process = executor.begin_execute(&config).await.unwrap();
            process.write_to_stdin(b"echo \"test\" ; exit").await.unwrap();
            process.close_stdin().await.unwrap();
            let status_code = process.await_exit().await.unwrap();
            assert_eq!(status_code, Some(0));
        }
        .boxed()
    })
    .await;
}

#[tokio::test]
async fn interactive_command_with_no_eof_returning_stdout() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/bash");
            config.redirect_stdout().redirect_stdin();
            let mut process = executor.begin_execute(&config).await.unwrap();
            process.write_to_stdin(b"echo \"test\" ; exit\n").await.unwrap();
            let process_output = process.await_exit_with_output().await.unwrap();
            assert_ok_execution(process_output, "test\n");
        }
        .boxed()
    })
    .await;
}

#[tokio::test]
async fn interactive_command_with_multiple_stdin_writes() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/bash");
            config.redirect_stdout().redirect_stdin().redirect_stderr();
            let mut process = executor.begin_execute(&config).await.unwrap();
            process.write_to_stdin(b"echo stdout\n").await.unwrap();
            process.write_to_stdin(b"echo stderr >&2\n").await.unwrap();
            process.close_stdin().await.unwrap();
            let process_output = process.await_exit_with_output().await.unwrap();
            assert_eq!(String::from_utf8(process_output.stdout).unwrap(), "stdout\n");
            assert_eq!(String::from_utf8(process_output.stderr).unwrap(), "stderr\n");
        }
        .boxed()
    })
    .await;
}

fn assert_ok_execution(process_output: FinishedLinuxProcessOutput, expectation: &str) {
    assert_eq!(process_output.status_code, Some(0));
    assert!(process_output.stderr.is_empty());
    assert_eq!(String::from_utf8(process_output.stdout).unwrap().as_str(), expectation);
}

// run the same test on 3 executor impls: native, openssh, russh
async fn executor_test<F>(function: F)
where
    F: FnOnce(Box<dyn LinuxExecutor + Send + '_>) -> BoxFuture<()>,
    F: Copy,
{
    let native = NativeLinux {};
    let russh_data = RusshData::setup().await;
    let openssh_data = OpensshData::setup().await;

    function(Box::new(native)).await;
    function(Box::new(russh_data.implementation)).await;
    function(Box::new(openssh_data.implementation)).await;
}

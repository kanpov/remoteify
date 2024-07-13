use common::{OpensshData, RusshData};
use futures::{future::BoxFuture, FutureExt};
use remoteify::{
    executor::{LinuxExecutor, LinuxProcessConfiguration},
    native::NativeLinux,
};
use tokio::sync::Mutex;
use uuid::Uuid;

mod common;

static SEQUENTIALITY_MUTEX: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn simple_command_outputting() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/echo");
            config.arg("--help").redirect_stdout();
            let process_output = executor.execute(&config).await.expect("Execution failed");
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
            let process_output = executor.execute(&config).await.expect("Execution failed");
            let stderr = String::from_utf8(process_output.stderr).unwrap();

            assert_ne!(process_output.status_code.expect("No status code provided"), 0);
            assert!(stderr.contains(id.to_string().as_str()));
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
            let mut process = executor.begin_execute(&config).await.expect("Spawn failed");
            process
                .write_to_stdin(b"echo \"test\" ; exit")
                .await
                .expect("Write failed");
            process.close_stdin().await.expect("EOF failed");
            let status_code = process.await_exit().await.expect("Awaiting failed");
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
            config.redirect_stdout();
            config.redirect_stdin();
            let mut process = executor.begin_execute(&config).await.unwrap();
            process.write_to_stdin(b"echo \"test\" ; exit\n").await.unwrap();
            let process_output = process.await_exit_with_output().await.unwrap();
            assert_eq!(process_output.status_code, Some(0));
            assert_eq!(String::from_utf8(process_output.stdout).unwrap(), "test\n");
            assert!(process_output.stderr.is_empty());
        }
        .boxed()
    })
    .await;
}

// run the same test on 3 executor impls: native, openssh, ru
async fn executor_test<F>(function: F)
where
    F: FnOnce(Box<dyn LinuxExecutor + Send + '_>) -> BoxFuture<()>,
    F: Copy,
{
    let guard = SEQUENTIALITY_MUTEX.lock().await;

    let native = NativeLinux {};
    function(Box::new(native)).await;

    let russh_data = RusshData::setup().await;
    function(Box::new(russh_data.implementation)).await;

    let openssh_data = OpensshData::setup().await;
    function(Box::new(openssh_data.implementation)).await;

    drop(guard);
}

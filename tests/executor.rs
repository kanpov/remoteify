use common::RusshData;
use futures::{future::BoxFuture, FutureExt};
use remoteify::{
    executor::{LinuxExecutor, LinuxProcessConfiguration},
    native::NativeLinux,
};
use uuid::Uuid;

mod common;

#[tokio::test]
async fn execution_with_only_stdout() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/dd");
            config.arg("--help").redirect_stdout();
            let process_output = executor.execute(&config).await.expect("Execution failed");
            let stdout = String::from_utf8(process_output.stdout.expect("No stdout provided")).unwrap();

            assert_eq!(process_output.status_code.expect("No status code provided"), 0);
            assert_eq!(process_output.stderr, None);
            assert!(stdout.contains("Copy a file, converting and formatting according to the operands."));
            assert!(stdout.contains("GNU coreutils online help: <https://www.gnu.org/software/coreutils/>"));
        }
        .boxed()
    })
    .await;
}

#[tokio::test]
async fn execution_with_only_stderr() {
    executor_test(|executor| {
        async move {
            let mut config = LinuxProcessConfiguration::new("/usr/bin/cat");
            let id = Uuid::new_v4();
            config.arg(format!("/tmp/{}", id));
            config.redirect_stderr();
            let process_output = executor.execute(&config).await.expect("Execution failed");
            let stderr = String::from_utf8(process_output.stderr.expect("No stderr provided")).unwrap();

            assert_ne!(process_output.status_code.expect("No status code provided"), 0);
            assert!(stderr.contains(id.to_string().as_str()));
        }
        .boxed()
    })
    .await;
}

async fn executor_test<F>(function: F)
where
    F: FnOnce(Box<dyn LinuxExecutor + Send + '_>) -> BoxFuture<()>,
    F: Copy,
{
    let native = NativeLinux {};
    function(Box::new(native)).await;

    let russh_data = RusshData::setup().await;
    function(Box::new(russh_data.implementation)).await;
}

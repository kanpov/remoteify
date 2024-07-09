use remoteify::{
    executor::{LinuxExecutor, LinuxProcessConfiguration},
    native::NativeLinux,
};
use uuid::Uuid;

mod common;

static IMPL: NativeLinux = NativeLinux {};

#[tokio::test]
async fn execution_with_only_stdout() {
    let mut config = LinuxProcessConfiguration::new("/usr/bin/dd");
    config.arg("--help").redirect_stdout();
    let process_output = IMPL.execute(&config).await.expect("Execution failed");
    let stdout = String::from_utf8(process_output.stdout.expect("No stdout provided")).unwrap();

    assert_eq!(process_output.status_code.expect("No status code provided"), 0);
    assert!(process_output.stderr.is_none());
    assert!(stdout.contains("Copy a file, converting and formatting according to the operands."));
    assert!(stdout.contains("GNU coreutils online help: <https://www.gnu.org/software/coreutils/>"));
}

#[tokio::test]
async fn execution_with_only_stderr() {
    let mut config = LinuxProcessConfiguration::new("/usr/bin/cat");
    let id = Uuid::new_v4();
    config.arg(format!("/tmp/{}", id));
    config.redirect_stderr();
    let process_output = IMPL.execute(&config).await.expect("Execution failed");
    let stderr = String::from_utf8(process_output.stderr.expect("No stderr provided")).unwrap();

    assert_ne!(process_output.status_code.expect("No status code provided"), 0);
    assert!(stderr.contains(id.to_string().as_str()));
}

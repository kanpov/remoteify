use lhf::{
    executor::{LinuxExecutor, LinuxProcess, LinuxProcessConfiguration},
    native::NativeLinux,
};

mod common;

static IMPL: NativeLinux = NativeLinux {};

#[tokio::test]
async fn t() {
    let mut config = LinuxProcessConfiguration::new("/usr/bin/sudo");
    config.arg("-S");
    config.arg("dd");
    config.arg("--help");
    config.redirect_stdout();
    config.redirect_stdin();

    let mut proc = IMPL.begin_execute(&config).await.expect("Failed to execute");
    proc.write_to_stdin(b"495762").await.expect("Failed stdin write");
    let output = proc
        .await_exit_with_output()
        .await
        .expect("Timed out waiting for exit likely");
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{stdout}");
}

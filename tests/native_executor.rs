use remoteify::{
    executor::{LinuxExecutor, LinuxProcess, LinuxProcessConfiguration},
    native::NativeLinux,
};

mod common;

static IMPL: NativeLinux = NativeLinux {};

#[tokio::test]
async fn t() {
    let mut config = LinuxProcessConfiguration::new("/usr/bin/dd");
    config.arg("--help");
    config.redirect_stdout();
    config.redirect_stdin();

    let proc = IMPL.begin_execute(&config).await.expect("Failed to execute");
    let output = proc.await_exit_with_output().await.unwrap();
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    println!("{stdout_str}");
}

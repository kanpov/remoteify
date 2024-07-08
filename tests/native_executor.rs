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

    let mut proc = IMPL.begin_execute(&config).await.expect("Failed to execute");
    proc.await_exit().await.unwrap();
    let po = proc.get_partial_output().unwrap();
    let stdout_str = String::from_utf8(po.stdout.unwrap()).unwrap();
    print!("{stdout_str}");
}

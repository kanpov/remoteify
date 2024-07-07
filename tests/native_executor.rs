use lhf::{
    executor::{LinuxExecutor, LinuxProcessConfiguration},
    native::NativeLinux,
};

mod common;

static IMPL: NativeLinux = NativeLinux {};

#[tokio::test]
async fn t() {
    let config = LinuxProcessConfiguration::new("/usr/bin/cat".into())
        .arg("--help".into())
        .redirect_stdout()
        .redirect_stderr()
        .clone();
    let received_output = IMPL.execute(config).await.expect("Failed to execute");
    let stdout = String::from_utf8(received_output.stdout).unwrap();
    println!("{stdout}");
    dbg!(received_output.stderr);
    dbg!(received_output.status_code);
}

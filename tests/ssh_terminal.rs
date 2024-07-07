use common::TestData;
use lhf::terminal::{LinuxTerminal, LinuxTerminalEvent, LinuxTerminalLauncher};

mod common;

#[tokio::test]
async fn t() {
    let test_data = TestData::setup().await;
    let terminal = test_data
        .implementation
        .launch_terminal_noninteractive()
        .await
        .expect("Call failed");
    terminal.run("echo q".into()).await.unwrap();
    terminal.send_input(b"root123", None).await.unwrap();
    terminal.send_eof().await.unwrap();
    loop {
        let event = terminal.await_next_event().await;
        match event {
            Some(LinuxTerminalEvent::ExtendedDataReceived { extended_data, ext: _ }) => {
                let output = String::from_utf8(extended_data).unwrap();
                println!("{output}");
            }
            Some(ev) => {
                dbg!(ev);
            }
            None => break,
        }
    }
}

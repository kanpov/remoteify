use std::io;

use async_trait::async_trait;

#[derive(Debug, Clone, Copy)]
pub enum LinuxTerminalEvent<'a> {
    EOFReceived,
    DataReceived {
        data: &'a [u8],
    },
    ExtendedDataReceived {
        ext: u32,
        extended_data: &'a [u8],
    },
    XonXoffAbilityReceived {
        can_perform_xon_xoff: bool,
    },
    ProcessExitedNormally {
        exit_status: u32,
    },
    ProcessExitedAfterSignal {
        signal: &'a str,
        core_dumped: bool,
        error_message: &'a str,
        lang_tag: &'a str,
    },
    WindowAdjusted {
        new_size: u32,
    },
    TerminalDisconnected,
}

#[async_trait]
pub trait LinuxTerminalEventReceiver: Send + Sync {
    async fn receive_event(&self, terminal_event: LinuxTerminalEvent);
}

#[async_trait]
pub trait LinuxTerminal {
    fn supports_event_receiver() -> bool {
        false
    }

    #[allow(unused_variables)]
    fn register_event_receiver(receiver: impl LinuxTerminalEventReceiver) {}

    fn unregister_event_receiver() {}
}

#[async_trait]
pub trait LinuxTerminalLauncher {
    async fn launch_terminal_noninteractive(&self) -> Result<impl LinuxTerminal, io::Error>;

    async fn launch_terminal_interactive(
        &self,
        terminal: &str,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<impl LinuxTerminal, io::Error>;
}

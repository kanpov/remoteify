use async_trait::async_trait;

#[derive(Debug, Clone, Copy)]
pub enum TerminalEvent<'a> {
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
    ProcessExited {
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
pub trait TerminalEventReceiver: Send {
    async fn receive_event(&self, terminal_event: TerminalEvent);
}

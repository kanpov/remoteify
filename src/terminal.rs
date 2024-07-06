use async_trait::async_trait;

pub enum TerminalEvent<'a> {
    EOFReceived,
    DataReceived {
        data: &'a [u8],
    },
    ExtendedDataReceived {
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
    CriticalFailure,
}

#[async_trait]
pub trait TerminalEventReceiver {
    async fn receive_event(terminal_event: &TerminalEvent);
}

use std::{error::Error, sync::Arc};

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum LinuxTerminalEvent {
    EOFReceived,
    DataReceived {
        data: Vec<u8>,
    },
    ExtendedDataReceived {
        ext: u32,
        extended_data: Vec<u8>,
    },
    XonXoffAbilityReceived {
        can_perform_xon_xoff: bool,
    },
    ProcessExitedNormally {
        exit_status: u32,
    },
    ProcessExitedAfterSignal {
        signal: String,
        core_dumped: bool,
        error_message: String,
        lang_tag: String,
    },
    WindowAdjusted {
        new_size: u32,
    },
    QueuedOperationSucceeded,
    QueuedOperationFailed,
    TerminalDisconnected,
}

#[derive(Debug, Clone)]
pub enum LinuxTerminalError {
    DHSInternalProblem,
    EventReceiverAlreadyExists,
    EventReceiverMissing,
    Other(Arc<Box<dyn Error>>),
}

impl LinuxTerminalError {
    pub(crate) fn other<E>(error: E) -> LinuxTerminalError
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        LinuxTerminalError::Other(Arc::new(error.into()))
    }
}

#[async_trait]
pub trait LinuxTerminalEventReceiver: Send + Sync {
    async fn receive_event(&self, terminal_event: LinuxTerminalEvent);
}

#[async_trait]
pub trait LinuxTerminal {
    #[allow(unused_variables)]
    async fn register_event_receiver<R>(&self, receiver: R) -> Result<(), LinuxTerminalError>
    where
        R: LinuxTerminalEventReceiver + 'static;

    async fn unregister_event_receiver(&self) -> Result<(), LinuxTerminalError>;

    async fn run(&self, command: String) -> Result<(), LinuxTerminalError>;

    async fn set_env_var(&self, name: String, value: String) -> Result<(), LinuxTerminalError>;

    async fn send_eof(&self) -> Result<(), LinuxTerminalError>;

    async fn send_signal(&self, signal: String) -> Result<(), LinuxTerminalError>;

    async fn send_input(&self, input: &[u8], ext: Option<u32>) -> Result<(), LinuxTerminalError>;

    async fn await_next_event(&self) -> Option<LinuxTerminalEvent>;

    async fn quit(&self) -> Result<(), LinuxTerminalError>;
}

#[async_trait]
pub trait LinuxTerminalLauncher {
    async fn launch_terminal_noninteractive(&self) -> Result<impl LinuxTerminal, LinuxTerminalError>;

    async fn launch_terminal_interactive(
        &self,
        terminal: &str,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<impl LinuxTerminal, LinuxTerminalError>;
}

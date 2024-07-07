## Module comparison

| Module     | Description                                    | Native | RuSSH |
|------------|------------------------------------------------|--------|-------|
| Filesystem | Access to the Linux filesystem                 | ✅      | ✅     |
| Network    | Access to forwarding TCP and Unix sockets      | ✅      | 🚧    |
| Terminal   | Reactive and async access to Linux terminals   | ❌      | ✅     |
| Executor   | A simple model for executing Linux programs    | 🚧     | 🚧    |
| HTTP       | Async HTTP client on Linux and not the host OS | 🚧     | 🚧    |

Implementation details:
- Filesystem: Tokio "async" I/O on native and SFTP on SSH
- Network: no-op on native and forwarding on SSH (`tcpip`, `streamlocal` extensions)
- Terminal: not planned on native due to process restrictions, simple execution / requested PTY on SSH
- Executor: Tokio async process on native and simple execution / requested PTY with less features on SSH
- HTTP: reqwest on native and remote-side raw curl on SSH

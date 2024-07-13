## Remoteify

Remoteify is an async Rust library that allows developers to build Linux-oriented libraries that run both on native Linux and remotely over SSH (or, potentially, other protocols in the future), without reinventing the wheel. Instead of building abstractions over a native filesystem or SFTP, over native process execution or SSH, over networking or socket forwarding for every library, Remoteify provides you with those abstractions out-of-the-box.

Three modules are currently present:
- **Filesystem**, for interacting with the Linux filesystem
- **Network**, for reverse-forwarding TCP/IP and Unix sockets (if necessary)
- **Executor**, for running and managing Linux processes

### How to use

#### As a library developer

Install the `remoteify` crate **without any non-default features**.

When needing the functionality of a module in your application, accept, for example, `impl LinuxFilesystem` or `impl LinuxNetwork` or multiple, like `impl LinuxFilesystem + LinuxNetwork`.

Avoid storing module implementations globally, as this won't allow users to use multiple remote Linux-es with your library!

#### As a library user

First, choose the implementation of Remoteify Linux you want to use in your code:

- Native, running on your Linux device (if you're developing on one). This is gated under the `native` feature of the `remoteify` crate
- `russh` + `russh-sftp` crates, running over SSH and SFTP. This is gated under the `russh` feature of the `remoteify` crate
- OpenSSH, running over SSH and SFTP as well but by interacting via openssh written in C. This is gated under the `openssh` feature of the `remoteify` crate
- A third party implementation not bundled in the crate. Import the third-party crate and use it according to its instructions in that case

Then, install the necessary crates. Now, create the implementation `struct` that implements the modules. For example, this would be `NativeLinux` for native impl and `RusshLinux` for RuSSH impl. For any library functions accepting a module implementation, pass in this `struct` and use the library according to your needs!

### Russh vs Openssh

- Russh is generally a bit faster due to no UDS communication (between the Rust program and the OpenSSH master socket)
- Russh executor doesn't need extra reads, but the performance implication is generally incredibly negligible
- Openssh is the only one that supports forwarding at the moment (calling `direct_forward` and `reverse_forward` with Russh will produce a `UnsupportedOperation` error)

As such, even though it'd be preferable to have a fully-featured native Rust SSH implementation, currently I recommend using OpenSSH instead. The forwarding limitation may be addressed in the future.

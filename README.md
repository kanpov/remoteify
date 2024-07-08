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
- RuSSH, running over SSH and SFTP to remote Linux using `russh` and `russh-sftp` crates. This is gated under the `ssh_russh` feature of the `remoteify` crate
- A third party implementation not bundled in the crate. Import the third-party crate and use it according to its instructions in that case

Then, install the necessary crates. Now, create the implementation `struct` that implements the modules. For example, this would be `NativeLinux` for native impl and `RusshLinux` for RuSSH impl. For any library functions accepting a module implementation, pass in this `struct` and use the library according to your needs!

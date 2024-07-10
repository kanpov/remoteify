use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use openssh::{Session, SessionBuilder, Stdio};
use openssh_sftp_client::{Sftp, SftpOptions};
use remoteify::{
    filesystem::{LinuxDirEntry, LinuxFileType},
    ssh_openssh::OpensshLinux,
    ssh_russh::{
        connection::{RusshAuthentication, RusshConnectionOptions},
        RusshLinux, RusshPtyOptions,
    },
};
use russh::{
    client::{self, Config, Handle, Msg},
    Channel,
};
use russh_keys::key::PublicKey;
use russh_sftp::client::SftpSession;
use testcontainers::{core::ContainerPort, runners::AsyncRunner, GenericImage};
use tokio::sync::OnceCell;
use uuid::Uuid;

pub fn gen_tmp_path() -> PathBuf {
    PathBuf::from(format!("/tmp/{}", Uuid::new_v4().to_string()))
}

#[allow(unused)]
pub fn gen_nested_tmp_path() -> PathBuf {
    PathBuf::from(format!(
        "/tmp/{}/{}",
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string()
    ))
}

#[allow(unused)]
static CONTAINER_PORT_CELL: OnceCell<u16> = OnceCell::const_new();

#[allow(unused)]
pub struct RusshData {
    pub ssh: Channel<Msg>,
    pub sftp: SftpSession,
    pub implementation: RusshLinux<AcceptingHandler>,
}

#[allow(unused)]
pub struct OpensshData {
    pub ssh: Session,
    pub sftp: Sftp,
    pub implementation: OpensshLinux,
}

impl RusshData {
    #[allow(unused)]
    pub async fn setup() -> RusshData {
        let ssh_port = get_ssh_port().await;

        let mut handle_option: Option<Handle<AcceptingHandler>> = None;
        loop {
            match client::connect(
                Arc::new(Config::default()),
                ("localhost", ssh_port),
                AcceptingHandler {},
            )
            .await
            {
                Ok(handle) => {
                    handle_option = Some(handle);
                    break;
                }
                Err(_) => {}
            }
        }

        let mut handle = handle_option.unwrap();
        handle
            .authenticate_password("root", "root123")
            .await
            .expect("Could not auth");
        let ssh_chan = handle.channel_open_session().await.expect("Could not open SSH channel");
        let sftp_chan = handle
            .channel_open_session()
            .await
            .expect("Could not open SFTP channel");
        sftp_chan
            .request_subsystem(true, "sftp")
            .await
            .expect("Could not request SFTP");
        let sftp_session = SftpSession::new(sftp_chan.into_stream())
            .await
            .expect("Could not open SFTP session");
        let mut impl_option: Option<RusshLinux<AcceptingHandler>> = None;

        loop {
            if let Ok(implementation) = RusshLinux::connect(
                AcceptingHandler {},
                RusshConnectionOptions {
                    host: "localhost".into(),
                    port: ssh_port,
                    username: "root".into(),
                    config: Config::default(),
                    authentication: RusshAuthentication::Password {
                        password: "root123".into(),
                    },
                },
                RusshPtyOptions {
                    terminal: "bash".into(),
                    col_width: 1000,
                    row_height: 1000,
                    pix_width: 0,
                    pix_height: 0,
                    terminal_modes: Vec::new(),
                },
            )
            .await
            {
                impl_option = Some(implementation);
                break;
            }
        }

        RusshData {
            ssh: ssh_chan,
            sftp: sftp_session,
            implementation: impl_option.unwrap(),
        }
    }

    #[allow(unused)]
    pub async fn init_file(&self, content: &str) -> PathBuf {
        let path = gen_tmp_path();
        self.sftp.create(conv_path(&path)).await.unwrap();
        self.sftp.write(conv_path(&path), content.as_bytes()).await.unwrap();
        path
    }

    #[allow(unused)]
    pub async fn assert_file(&self, path: &PathBuf, expected_content: &str) {
        let actual_content = String::from_utf8(self.sftp.read(conv_path(&path)).await.unwrap()).unwrap();
        assert_eq!(actual_content, expected_content);
    }
}

impl OpensshData {
    #[allow(unused)]
    pub async fn setup() -> OpensshData {
        let ssh_port = get_ssh_port().await;
        let owned_session;

        loop {
            match mk_openssh_session(&ssh_port).await {
                Ok(session) => {
                    owned_session = session;
                    break;
                }
                Err(_) => {}
            }
        }

        let given_session = mk_openssh_session(&ssh_port)
            .await
            .expect("Could not establish given session");

        let mut child = owned_session
            .subsystem("sftp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .await
            .expect("Could not make sftp child");
        let owned_sftp = Sftp::new(
            child.stdin().take().unwrap(),
            child.stdout().take().unwrap(),
            SftpOptions::default(),
        )
        .await
        .expect("Could not make sftp");

        let implementation = OpensshLinux::new(given_session, SftpOptions::default())
            .await
            .expect("Could not establish impl");

        OpensshData {
            ssh: owned_session,
            sftp: owned_sftp,
            implementation,
        }
    }
}

async fn mk_openssh_session(ssh_port: &u16) -> Result<Session, openssh::Error> {
    let builder = SessionBuilder::default();
    let str_dest = format!("ssh://root@localhost:{}", ssh_port);
    let (builder, dest) = builder.resolve(str_dest.as_str());
    let tempdir = builder.launch_master(dest).await?;

    Ok(Session::new_process_mux(tempdir))
}

async fn get_ssh_port() -> u16 {
    let ssh_port = CONTAINER_PORT_CELL
        .get_or_init(|| async {
            std::env::set_var("TESTCONTAINERS_COMMAND", "keep");
            let container = GenericImage::new("ssh_server", "latest")
                .with_exposed_port(ContainerPort::Tcp(22))
                .start()
                .await
                .expect("Could not start SSH container");
            let ports = container.ports().await.expect("Could not get SSH container ports");
            let port = ports
                .map_to_host_port_ipv4(ContainerPort::Tcp(22))
                .expect("Could not get SSH container port corresponding to 22");
            port
        })
        .await;
    *ssh_port
}

#[allow(unused)]
pub fn conv_path(path: &PathBuf) -> String {
    path.to_str().unwrap().into()
}

#[allow(unused)]
pub fn conv_path_non_buf(path: &Path) -> String {
    path.to_str().unwrap().into()
}

#[allow(unused)]
pub fn entries_contain(entries: &Vec<LinuxDirEntry>, expected_type: LinuxFileType, expected_path: &PathBuf) {
    assert!(entries.iter().any(|entry| {
        matches!(entry.file_type(), expected_type)
            && entry.path().as_os_str() == expected_path.as_os_str()
            && entry.name().as_str() == expected_path.file_name().unwrap()
    }))
}

#[derive(Debug)]
pub struct AcceptingHandler {}

#[async_trait]
impl client::Handler for AcceptingHandler {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _server_public_key: &PublicKey) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

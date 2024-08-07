use std::{
    ffi::{OsStr, OsString},
    sync::Arc,
};

use async_trait::async_trait;
use openssh::{Session, SessionBuilder};
use openssh_sftp_client::{Sftp, SftpOptions};
use remoteify::{
    filesystem::{LinuxDirEntry, LinuxFileType},
    impl_openssh::OpensshLinux,
    impl_russh::{
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

pub fn gen_tmp_path() -> OsString {
    format!("/tmp/{}", Uuid::new_v4().to_string()).into()
}

#[allow(unused)]
pub fn gen_nested_tmp_path() -> (OsString, OsString) {
    let id1 = Uuid::new_v4().to_string();
    let id2 = Uuid::new_v4().to_string();

    (format!("/tmp/{}", id1).into(), format!("/tmp/{}/{}", id1, id2).into())
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
    pub ssh: Arc<Session>,
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
    pub async fn init_file(&self, content: &str) -> OsString {
        let path = gen_tmp_path();
        self.sftp.create(path.to_string_lossy()).await.unwrap();
        self.sftp
            .write(path.to_string_lossy(), content.as_bytes())
            .await
            .unwrap();
        path
    }

    #[allow(unused)]
    pub async fn assert_file(&self, path: &OsStr, expected_content: &str) {
        let actual_content = String::from_utf8(self.sftp.read(path.to_string_lossy()).await.unwrap()).unwrap();
        assert_eq!(actual_content, expected_content);
    }
}

impl OpensshData {
    #[allow(unused)]
    pub async fn setup() -> OpensshData {
        let ssh_port = get_ssh_port().await;

        let owned_session_arc = Arc::new(mk_openssh_session(&ssh_port).await.unwrap());
        let owned_sftp = Sftp::from_clonable_session(owned_session_arc.clone(), SftpOptions::default())
            .await
            .unwrap();

        let given_session = mk_openssh_session(&ssh_port).await.unwrap();
        let implementation = OpensshLinux::with_new_sftp(given_session, SftpOptions::default())
            .await
            .unwrap();

        OpensshData {
            ssh: owned_session_arc,
            sftp: owned_sftp,
            implementation,
        }
    }

    #[allow(unused)]
    pub async fn assert_file(&self, path: &OsStr, expected_content: &str) {
        let content_buf = self.sftp.fs().read(&path).await.unwrap();
        let content_str = String::from_utf8(content_buf.to_vec()).unwrap();
        assert_eq!(content_str, expected_content);
    }

    #[allow(unused)]
    pub async fn assert_file_exists(&self, path: &OsStr, exists: bool) {
        assert_eq!(self.sftp.fs().metadata(&path).await.is_ok(), exists);
    }

    #[allow(unused)]
    pub async fn assert_dir_exists(&self, path: &OsStr, exists: bool) {
        let metadata = self.sftp.fs().metadata(&path).await;
        assert_eq!(exists, metadata.is_ok());

        if exists {
            assert!(metadata.unwrap().file_type().unwrap().is_dir());
        }
    }
}

async fn mk_openssh_session(ssh_port: &u16) -> Result<Session, openssh::Error> {
    let builder = SessionBuilder::default();
    let str_dest = format!("ssh://root@localhost:{}", ssh_port);
    let (builder, dest) = builder.resolve(str_dest.as_str());
    let tempdir;

    loop {
        if let Ok(td) = builder.launch_master(dest).await {
            tempdir = td;
            break;
        }
    }

    let session = Session::new_process_mux(tempdir);

    Ok(session)
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
pub fn entries_contain(entries: &Vec<LinuxDirEntry>, expected_type: LinuxFileType, expected_path: &OsStr) {
    assert!(entries
        .iter()
        .any(|entry| { matches!(entry.file_type, expected_type) && entry.path.as_os_str() == expected_path }))
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

use std::{path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use lhf::ssh_russh::{
    connection::{RusshAuthentication, RusshConnectionOptions},
    RusshLinux,
};
use russh::{
    client::{self, Config, Msg},
    Channel,
};
use russh_keys::key::PublicKey;
use russh_sftp::client::SftpSession;
use testcontainers::{core::ContainerPort, runners::AsyncRunner, ContainerAsync, GenericImage};
use uuid::Uuid;

pub fn get_tmp_path() -> PathBuf {
    PathBuf::from(format!("/tmp/{}", Uuid::new_v4().to_string()))
}

pub struct TestData {
    pub ssh: Channel<Msg>,
    pub sftp: SftpSession,
    pub implementation: RusshLinux<TestHandler>,
    _container: ContainerAsync<GenericImage>,
}

impl TestData {
    pub async fn setup() -> TestData {
        let container = GenericImage::new("ssh_server", "latest")
            .with_exposed_port(ContainerPort::Tcp(22))
            .start()
            .await
            .expect("Could not start SSH container");
        let ports = container
            .ports()
            .await
            .expect("Could not get SSH container ports");
        let ssh_port = ports
            .map_to_host_port_ipv4(ContainerPort::Tcp(22))
            .expect("Could not get SSH container port corresponding to 22");
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut handle = client::connect(
            Arc::new(Config::default()),
            ("localhost", ssh_port),
            TestHandler {},
        )
        .await
        .expect("Could not connect");

        handle
            .authenticate_password("root", "root123")
            .await
            .expect("Could not auth");
        let ssh_chan = handle
            .channel_open_session()
            .await
            .expect("Could not open SSH channel");
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
        let implementation = RusshLinux::connect(
            TestHandler {},
            RusshConnectionOptions {
                host: "localhost".into(),
                port: ssh_port,
                username: "root".into(),
                config: Config::default(),
                authentication: RusshAuthentication::Password {
                    password: "root123".into(),
                },
            },
        )
        .await
        .expect("Could not establish impl");

        TestData {
            ssh: ssh_chan,
            sftp: sftp_session,
            implementation,
            _container: container,
        }
    }

    pub async fn init_file(&self, content: &str) -> PathBuf {
        let path = get_tmp_path();
        self.sftp.create(conv_path(&path)).await.unwrap();
        self.sftp
            .write(conv_path(&path), content.as_bytes())
            .await
            .unwrap();
        path
    }

    pub async fn assert_file(&self, path: &PathBuf, expected_content: &str) {
        let actual_content =
            String::from_utf8(self.sftp.read(conv_path(&path)).await.unwrap()).unwrap();
        assert_eq!(actual_content, expected_content);
    }
}

pub fn conv_path(path: &PathBuf) -> String {
    path.to_str().unwrap().into()
}

#[derive(Debug)]
pub struct TestHandler {}

#[async_trait]
impl client::Handler for TestHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

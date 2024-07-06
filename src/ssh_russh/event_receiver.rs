use async_trait::async_trait;
use russh::client;
use russh_keys::key;

#[async_trait]
pub trait RusshEventReceiver: Send {
    #[allow(unused_variables)]
    async fn check_server_key(&mut self, server_public_key: &key::PublicKey) -> Result<bool, russh::Error> {
        Ok(true)
    }
}

pub(super) struct DelegatingHandler<R>
where
    R: RusshEventReceiver,
{
    pub russh_handler: R,
}

#[async_trait]
impl<R> client::Handler for DelegatingHandler<R>
where
    R: RusshEventReceiver,
{
    type Error = russh::Error;

    async fn check_server_key(&mut self, server_public_key: &key::PublicKey) -> Result<bool, Self::Error> {
        self.russh_handler.check_server_key(server_public_key).await
    }
}

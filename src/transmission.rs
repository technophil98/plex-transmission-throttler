use std::env;

use anyhow::{anyhow, Context};
use transmission_rpc::types::{BasicAuth, SessionSetArgs};
use transmission_rpc::TransClient;

pub trait TransmissionClient {
    async fn enable_transmission_alt_speed(&mut self) -> anyhow::Result<()>;
    async fn disable_transmission_alt_speed(&mut self) -> anyhow::Result<()>;
}

pub fn new_transmission_client() -> anyhow::Result<TransClient> {
    let transmission_url = env::var("TRANSMISSION_URL")
        .context("TRANSMISSION_URL environment variable should be set.")?;
    let user = env::var("TRANSMISSION_USERNAME")
        .context("TRANSMISSION_USERNAME environment variable should be set.")?;
    let password = env::var("TRANSMISSION_PASSWORD")
        .context("TRANSMISSION_PASSWORD environment variable should be set.")?;
    let basic_auth = BasicAuth { user, password };

    let transmission_url = transmission_url
        .parse()
        .context("while parsing transmission URL")?;
    let client = TransClient::with_auth(transmission_url, basic_auth);

    Ok(client)
}

impl TransmissionClient for TransClient {
    async fn enable_transmission_alt_speed(&mut self) -> anyhow::Result<()> {
        set_transmission_alt_speed(self, true)
            .await
            .context("Cannot enable alt speed")
    }

    async fn disable_transmission_alt_speed(&mut self) -> anyhow::Result<()> {
        set_transmission_alt_speed(self, false)
            .await
            .context("Cannot disable alt speed")
    }
}

pub async fn set_transmission_alt_speed(
    transmission_client: &mut TransClient,
    enable_alt_speed: bool,
) -> anyhow::Result<()> {
    transmission_client
        .session_set(SessionSetArgs {
            alt_speed_enabled: Some(enable_alt_speed),
            ..SessionSetArgs::default()
        })
        .await
        .map(|_| ())
        .map_err(|e| anyhow!(e))
}

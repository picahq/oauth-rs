use dotenvy::dotenv;
use envconfig::Envconfig;
use oauth_refresh::{refresh, AppState, Refresh, RefreshConfig};
use osentities::telemetry::{get_subscriber, init_subscriber};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let suscriber = get_subscriber("oauth-refresh".into(), "info".into(), std::io::stdout, None);
    init_subscriber(suscriber);

    let configuration = RefreshConfig::init_from_env()?;

    tracing::info!(
        "Starting application with configuration: {}{:#?}{}",
        "\n",
        &configuration,
        "\n"
    );
    let state = AppState::try_from(configuration.clone()).await?;

    let sleep_timer = Duration::from_secs(configuration.sleep_timer());
    let refresh_before = configuration.refresh_before();

    loop {
        let res = refresh(
            Refresh::new(refresh_before),
            state.connections().clone(),
            state.secrets().clone(),
            state.oauths().clone(),
            state.client().clone(),
            state.metrics().clone(),
        )
        .await;
        if let Err(e) = res {
            tracing::warn!("Failed to send refresh message: {:?}", e);
        }

        tracing::info!("Sleeping for {} seconds", sleep_timer.as_secs());
        tokio::time::sleep(sleep_timer).await;
    }
}

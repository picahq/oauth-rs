use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use oauth_api::{Application, Configuration};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let suscriber = get_subscriber("oauth-api".into(), "info".into(), std::io::stdout);
    init_subscriber(suscriber);

    let configuration = Configuration::init_from_env()?;

    let address = configuration.server().app_url().to_string();
    let application = Application::start(&configuration).await?;

    tracing::info!("Starting server at {}", &address);
    application.spawn().await?;

    Ok(())
}

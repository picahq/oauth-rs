use dotenvy::dotenv;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use oauth_api::{Application, Configuration, OAuthConfig, ServerConfig};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let suscriber = get_subscriber("oauth_api".into(), "info".into(), std::io::stdout);
    init_subscriber(suscriber);

    let oauth = OAuthConfig::load().expect("Failed to read configuration.");
    let server = ServerConfig::load().expect("Failed to read configuration.");
    let configuration = Configuration::new(oauth, server);

    let address = configuration.server().app_url().to_string();
    let application = Application::start(&configuration).await?;

    tracing::info!("Starting server at {}", &address);
    application.spawn().await?;

    Ok(())
}

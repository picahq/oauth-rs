mod algebra;
mod domain;
mod service;

pub use algebra::*;
pub use domain::*;
pub use service::*;

use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{
    dev::Server,
    web::{scope, Data},
    App, HttpServer,
};
use actix_web_lab::middleware::from_fn;
use anyhow::Context;
use std::{net::TcpListener, time::Duration};

pub const PREFIX: &str = "/v1";
pub const INTEGRATION_PREFIX: &str = "/integration";

pub struct Application {
    port: u16,
    server: Server,
    task: Task,
}

impl Application {
    pub async fn start(configuration: &Configuration) -> Result<Self, anyhow::Error> {
        tracing::info!(
            "Starting application with configuration: {}{:#?}{}",
            "\n",
            &configuration,
            "\n"
        );
        let address = format!(
            "{}:{}",
            configuration.server().host(),
            configuration.server().port()
        );
        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr()?.port();
        let state = AppState::try_from(configuration.clone()).await?;

        let sleep_timer = Duration::from_secs(configuration.oauth().sleep_timer());
        let refresh_before = configuration.oauth().refresh_before();
        let refresh_actor = state.refresh_actor().clone();
        let task = Box::pin(async move {
            loop {
                let message = Refresh::new(refresh_before);
                let res = refresh_actor.send(message).await;

                if let Err(e) = res {
                    tracing::warn!("Failed to send refresh message: {:?}", e);
                }

                tracing::info!("Sleeping for {} seconds", sleep_timer.as_secs());
                tokio::time::sleep(sleep_timer).await;
            }
        });

        let server = run(listener, configuration.clone(), state).await?;

        Ok(Self { port, server, task })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn handler(self) -> (Server, Task) {
        (self.server, self.task)
    }

    pub async fn spawn(self) -> Result<(), anyhow::Error> {
        let (server, task) = self.handler();
        let task = tokio::spawn(task);
        let http = tokio::spawn(server);

        tokio::select! {
            res = http => {
                res.context("Failed to spawn http application.")?.context("Failed to spawn http application.")
            },
            res = task => {
                res.context("Failed to spawn background task.")
            }
        }
    }
}

async fn run(
    listener: TcpListener,
    configuration: Configuration,
    state: AppState,
) -> Result<Server, anyhow::Error> {
    let governor = GovernorConfigBuilder::default()
        .per_second(configuration.server().burst_rate_limit())
        .permissive(configuration.server().is_development())
        .burst_size(configuration.server().burst_size_limit())
        .finish()
        .context("Failed to create governor.")?;

    let server = HttpServer::new(move || {
        let trace: Tracer = Tracer::default();
        App::new()
            .wrap(trace.tracer())
            .wrap(
                Cors::default()
                    .allowed_methods(vec!["GET", "POST"])
                    .allow_any_origin()
                    .allow_any_header()
                    .supports_credentials()
                    .max_age(3600),
            )
            .wrap(Governor::new(&governor))
            .service(
                scope(&(PREFIX.to_owned() + INTEGRATION_PREFIX)) // /v1/integration
                    .wrap(from_fn(auth_middleware))
                    .service(trigger_refresh)
            )
            .service(scope(PREFIX).service(health_check)) // /v1
            .app_data(Data::new(state.clone()))
    })
    .listen(listener)?
    .run();

    Ok(server)
}

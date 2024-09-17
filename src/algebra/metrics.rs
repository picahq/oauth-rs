use metrics_exporter_prometheus::PrometheusBuilder;

pub const SUCCESSFULLY_REFRESHED_GAUGE: &str = "successfully_refreshed";
pub const FAILED_TO_REFRESH_GAUGE: &str = "failed_to_refresh";
pub const REFRESH_TOTAL: &str = "refresh_total";

#[derive(Clone, Debug)]
pub struct Metrics {
    is_installed: bool,
}

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let metric = PrometheusBuilder::new()
            .install()
            .map_err(|e| {
                tracing::error!("Failed to install prometheus exporter: {}", e);
            })
            .ok();

        if metric.is_some() {
            metrics::describe_gauge!(
                SUCCESSFULLY_REFRESHED_GAUGE,
                "The number of successfully refreshed connections"
            );

            metrics::describe_gauge!(
                FAILED_TO_REFRESH_GAUGE,
                "The number of failed to refresh connections"
            );

            metrics::describe_gauge!(REFRESH_TOTAL, "The total number of refreshes");

            Ok(Self { is_installed: true })
        } else {
            Ok(Self {
                is_installed: false,
            })
        }
    }

    pub fn add_refreshed(&self, value: u64) {
        if self.is_installed {
            metrics::increment_gauge!(SUCCESSFULLY_REFRESHED_GAUGE, value as f64);
            metrics::increment_gauge!(REFRESH_TOTAL, value as f64);
        }
    }

    pub fn add_failed_to_refresh(&self, value: u64) {
        if self.is_installed {
            metrics::increment_gauge!(FAILED_TO_REFRESH_GAUGE, value as f64);
            metrics::increment_gauge!(REFRESH_TOTAL, value as f64);
        }
    }
}

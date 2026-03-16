use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

pub fn install_prometheus_recorder() -> Result<PrometheusHandle, String> {
    PrometheusBuilder::new()
        .install_recorder()
        .map_err(|e| format!("failed to install prometheus recorder: {e}"))
}

use prometheus::{self, default_registry, Encoder, IntGauge, Opts, TextEncoder};
use std::net::SocketAddr;
use std::str::FromStr;
use tonic::codec::CompressionEncoding;
use tonic::{Request, Response, Status};
use tracing::info;

mod pb {
    tonic::include_proto!("prometheus");
}

use pb::metrics_server::{Metrics, MetricsServer};
use pb::{MetricsRequest, MetricsResponse};

lazy_static::lazy_static! {
    static ref APP_INSTANCES_METRIC: IntGauge = {
        let metric = IntGauge::with_opts(Opts::new("application_instance", "Number of instances of this application")
            .const_label("app", "test-server")
            .const_label("app_version", env!("CARGO_PKG_VERSION"))
        ).unwrap();
        default_registry().register(Box::new(metric.clone())).unwrap();
        metric.inc();
        metric
    };

    static ref SCRAPE_COUNT_METRIC: IntGauge = {
        let metric = IntGauge::with_opts(Opts::new("scrape_count", "Number of metrics scrape requests")).unwrap();
        default_registry().register(Box::new(metric.clone())).unwrap();
        metric
    };
}

#[derive(Debug)]
pub struct PrometheusService {}

impl PrometheusService {
    pub fn new_server() -> MetricsServer<Self> {
        MetricsServer::new(Self {})
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }
}

#[tonic::async_trait]
impl Metrics for PrometheusService {
    async fn metrics(
        &self,
        _request: Request<MetricsRequest>,
    ) -> Result<Response<MetricsResponse>, Status> {
        info!("Prometheus metrics request received");
        SCRAPE_COUNT_METRIC.inc();

        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();

        let metric_families = prometheus::gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        let output = String::from_utf8(buffer.clone()).unwrap();
        Ok(Response::new(MetricsResponse { text: output }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Needs to be initialized once.
    let _ = APP_INSTANCES_METRIC.get();

    tonic::transport::Server::builder()
        .add_service(PrometheusService::new_server())
        .serve(SocketAddr::from_str("127.0.0.1:11000").unwrap())
        .await?;
    Ok(())
}

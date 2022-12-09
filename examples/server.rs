use prometheus::{self, default_registry, Encoder, IntGauge, Opts, TextEncoder};
use std::net::SocketAddr;
use std::str::FromStr;
use tonic::codec::CompressionEncoding;
use tonic::{Request, Response, Status};
use tracing::info;
use tracing_subscriber::EnvFilter;

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

struct PrometheusService {}

#[tonic::async_trait]
impl Metrics for PrometheusService {
    async fn metrics(
        &self,
        _request: Request<MetricsRequest>,
    ) -> Result<Response<MetricsResponse>, Status> {
        info!("Serving metrics ...");
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
    dotenv::dotenv().ok();
    initialize_logging();

    // Needs to be initialized once.
    let _ = APP_INSTANCES_METRIC.get();

    let matches = build_command().get_matches();
    let grpc_endpoint: &SocketAddr = matches.get_one("grpc_endpoint").unwrap();

    tonic::transport::Server::builder()
        .add_service(
            MetricsServer::new(PrometheusService {})
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip),
        )
        .serve(grpc_endpoint.clone())
        .await?;
    Ok(())
}

pub fn build_command() -> clap::Command {
    use clap::{Arg, Command};

    return Command::new("Prometheus Metrics gRPC Proxy")
        .version("0.1.0")
        .author("Markus Mayer <widemeadows@gmail.com>")
        .about("Read Prometheus Metrics")
        .arg(
            Arg::new("grpc_endpoint")
                .short('b')
                .long("bind")
                .env("GRPC_SERVER_BIND_ENDPOINT")
                .value_name("ENDPOINT")
                .default_value("127.0.0.1:50051")
                .long_help("The socket address to bind to")
                .num_args(1)
                .value_parser(grpc_endpoint)
                .help_heading("gRPC Endpoint"),
        );

    //noinspection DuplicatedCode
    fn grpc_endpoint(s: &str) -> Result<SocketAddr, String> {
        match SocketAddr::from_str(s) {
            Ok(addr) => Ok(addr),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Initializes the tracing and logging system.
///
/// This method uses the default environment filter to configure logging.
/// Please use the `RUST_LOG` environment variable to tune.
fn initialize_logging() {
    let filter = EnvFilter::builder()
        .with_default_directive(tracing::metadata::LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt()
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(true)
        .with_target(true)
        .with_env_filter(filter)
        .init();
}

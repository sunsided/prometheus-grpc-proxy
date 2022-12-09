use crate::pb::metrics_client::MetricsClient;
use crate::pb::MetricsRequest;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use clap::parser::ValuesRef;
use clap::ArgAction;
use std::net::SocketAddr;
use std::str::FromStr;
use tonic::codec::CompressionEncoding;
use tonic::transport::{Channel, Uri};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

mod pb {
    tonic::include_proto!("prometheus");
}

struct AppState {
    /// The gRPC channel to the upstream service.
    channel: Channel,
}

/// This endpoint always returns 200 OK in order to prevent
/// accidental startup issues on Kubernetes with the default
/// readiness and liveness probes.
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("# Use /metrics to obtain Prometheus metrics from this service.")
}

/// This is the default Prometheus endpoint.
#[get("/metrics")]
async fn metrics(data: web::Data<AppState>) -> impl Responder {
    debug!("Fetching metrics from gRPC service ...");

    let mut client = MetricsClient::new(data.channel.clone())
        .send_compressed(CompressionEncoding::Gzip)
        .accept_compressed(CompressionEncoding::Gzip);

    let request = tonic::Request::new(MetricsRequest::default());
    let response = client.metrics(request).await.unwrap().into_inner();

    HttpResponse::Ok().body(response.text)
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let matches = build_command().get_matches();
    let logging_style: &LoggingStyle = matches.get_one("logging_style").unwrap();
    let bind_endpoints: ValuesRef<SocketAddr> = matches.get_many("bind_endpoint").unwrap();
    let grpc_endpoint: &Uri = matches.get_one("grpc_endpoint").unwrap();

    initialize_logging(*logging_style);
    let channel = build_grpc_channel(grpc_endpoint).await?;
    run_server(channel, bind_endpoints).await?;

    Ok(())
}

async fn build_grpc_channel(grpc_endpoint: &Uri) -> anyhow::Result<Channel> {
    let channel = match Channel::builder(grpc_endpoint.clone()).connect().await {
        Ok(channel) => channel,
        Err(e) => {
            eprintln!(
                "Failed to connect to the metric endpoint at {}",
                grpc_endpoint
            );
            eprintln!("Got error {:?}", e);
            return Err(e.into());
        }
    };

    Ok(channel)
}

async fn run_server(
    channel: Channel,
    bind_endpoints: ValuesRef<'_, SocketAddr>,
) -> anyhow::Result<()> {
    // State created here to avoid multiple construction.
    let state = web::Data::new(AppState { channel });

    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(index)
            .service(metrics)
    });

    for endpoint in bind_endpoints.into_iter() {
        info!("Binding to {endpoint}", endpoint = endpoint.to_string());
        server = server.bind(endpoint)?
    }

    info!("Starting server ...");
    server.run().await?;

    info!("Server stopped");
    Ok(())
}

#[derive(Debug, Copy, Clone)]
pub enum LoggingStyle {
    /// Uses compact logging.
    Compact,
    /// Uses JSON formatted logging
    Json,
}

/// Initializes the tracing and logging system.
///
/// This method uses the default environment filter to configure logging.
/// Please use the `RUST_LOG` environment variable to tune.
fn initialize_logging(style: LoggingStyle) {
    let filter = EnvFilter::builder()
        .with_default_directive(tracing::metadata::LevelFilter::INFO.into())
        .from_env_lossy();

    let formatter = tracing_subscriber::fmt()
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(true)
        .with_target(true)
        .with_env_filter(filter);

    match style {
        LoggingStyle::Compact => formatter.init(),
        LoggingStyle::Json => formatter.json().init(),
    }
}

pub fn build_command() -> clap::Command {
    use clap::{Arg, Command};

    return Command::new("Prometheus Metrics gRPC Proxy")
        .version("0.1.0")
        .author("Markus Mayer <widemeadows@gmail.com>")
        .about("Read Prometheus Metrics")
        .arg(
            Arg::new("bind_endpoint")
                .short('b')
                .long("bind")
                .env("HTTP_SERVER_BIND_ENDPOINT")
                .value_name("BIND_ENDPOINT")
                .default_value("127.0.0.1:8080")
                .long_help("The endpoint to bind to")
                .num_args(1)
                .action(ArgAction::Append)
                .value_parser(bind_endpoint)
                .help_heading("HTTP Binding"),
        )
        .arg(
            Arg::new("grpc_endpoint")
                .short('e')
                .long("endpoint")
                .env("GRPC_CLIENT_ENDPOINT")
                .value_name("ENDPOINT")
                .default_value("http://127.0.0.1:50051")
                .long_help("The gRPC endpoint to connect to")
                .num_args(1)
                .value_parser(grpc_endpoint)
                .help_heading("gRPC Endpoint"),
        )
        .arg(
            Arg::new("logging_style")
                .short('l')
                .long("log")
                .env("HTTP_SERVER_LOG_STYLE")
                .value_name("STYLE")
                .default_value("simple")
                .help("The logging style to use (simple, json)")
                .num_args(1)
                .value_parser(logging_style)
                .help_heading("Logging"),
        );

    fn bind_endpoint(s: &str) -> Result<SocketAddr, String> {
        match SocketAddr::from_str(s) {
            Ok(addr) => Ok(addr),
            Err(e) => Err(e.to_string()),
        }
    }

    fn grpc_endpoint(s: &str) -> Result<Uri, String> {
        match Uri::from_str(s) {
            Ok(uri) => Ok(uri),
            Err(e) => Err(e.to_string()),
        }
    }

    fn logging_style(s: &str) -> Result<LoggingStyle, String> {
        match s {
            "simple" => Ok(LoggingStyle::Compact),
            "compat" => Ok(LoggingStyle::Compact),
            "json" => Ok(LoggingStyle::Json),
            _ => Err(String::from("Either simple or json must be specified")),
        }
    }
}

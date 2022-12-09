use crate::pb::metrics_client::MetricsClient;
use crate::pb::MetricsRequest;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use std::error::Error;
use std::str::FromStr;
use tonic::codec::CompressionEncoding;
use tonic::transport::{Channel, Uri};

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
    let grpc_endpoint: &Uri = matches.get_one("grpc_endpoint").unwrap();

    let channel = build_grpc_channel(grpc_endpoint).await?;
    run_server(channel).await?;

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

async fn run_server(channel: Channel) -> anyhow::Result<()> {
    // State created here to avoid multiple construction.
    let state = web::Data::new(AppState { channel });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(index)
            .service(metrics)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
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
                .short('e')
                .long("endpoint")
                .env("GRPC_SERVER_ENDPOINT")
                .value_name("ENDPOINT")
                .default_value("http://localhost:50051")
                .long_help("The gRPC endpoint scheme to connect to")
                .num_args(1)
                .value_parser(endpoint)
                .help_heading("gRPC Endpoint"),
        );

    fn endpoint(s: &str) -> Result<Uri, String> {
        match Uri::from_str(s) {
            Ok(uri) => Ok(uri),
            Err(e) => Err(e.to_string()),
        }
    }
}

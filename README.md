# Prometheus gRPC/HTTP Bridge

This little companion service provides an HTTP bridge for a trivial (non-standard) Prometheus gRPC 
metrics protocol over gRPC (see [prometheus.proto](protos/prometheus/prometheus.proto)):

```protobuf
syntax = "proto3";
package prometheus;

service Metrics {
  rpc Metrics(MetricsRequest) returns (MetricsResponse) {}
}

message MetricsRequest {}

message MetricsResponse {
  string text = 1;
}
```

The main purpose of this project is to allow a regular Tonic server to provide Prometheus metrics without
the hassle of providing an additional HTTP server endpoint.

The service is available as Docker image from [`sunside/prometheus-grpc-proxy`](https://hub.docker.com/repository/docker/sunside/prometheus-grpc-proxy).
See [`docker-compose.yml`](docker-compose.yml) for a usage example.

## Command-Line Arguments

* `--bind <BIND_ENDPOINT>` (or `HTTP_SERVER_BIND_ENDPOINT` environment variable)
  Specifies the endpoint to bind the HTTP server to. Can be specified multiple times
  and defaults to `127.0.0.1:8080`.
* `--endpoint <GRPC_ENDPOINT>` (or `GRPC_CLIENT_ENDPOINT` environment variable)
  Specifies the gRPC endpoint to connect to. Must include the protocol; defaults to `http://127.0.0.1:50051`.
* `--log <STYLE>` (or `HTTP_SERVER_LOG_STYLE` environment variable)
  Either `simple` or `json`, selects the logging style; defaults to `simple`.

## Testing the service

A [`Dockerfile`](Dockerfile) and [`docker-compose.yml`](docker-compose.yml) definition are provided to test the service.
After executing

```bash
$ docker compose up
```

you should be able to visit the Prometheus endpoint in your browser at [`http://localhost:8080/metrics`](http://localhost:8080/metrics).

To build the Proxy image yourself, run

```bash
$ docker build --tag sunside/prometheus-grpc-proxy:latest --target prometheus-grpc-proxy .
```

## Extension points

[OpenMetrics] gRPC support may be added at a later point.

[OpenMetrics]: https://github.com/OpenObservability/OpenMetrics/blob/main/proto/openmetrics_data_model.proto

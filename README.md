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

## Extension points

[OpenMetrics] gRPC support may be added at a later point.

[OpenMetrics]: https://github.com/OpenObservability/OpenMetrics/blob/main/proto/openmetrics_data_model.proto

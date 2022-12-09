FROM rust:1.65-bullseye as builder

RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/proxy
COPY examples examples
COPY src src
COPY protos protos
COPY build.rs .
COPY Cargo.toml .
COPY Cargo.lock .

RUN cargo install --bins --examples --path .

# The proxy server image.
FROM debian:bullseye-slim as prometheus-grpc-proxy

COPY --from=builder /usr/local/cargo/bin/prometheus-grpc-proxy /usr/local/bin/prometheus-grpc-proxy

ENV HTTP_SERVER_BIND_ENDPOINT=0.0.0.0:80
ENV GRPC_SERVER_BIND_ADDRESS=127.0.0.1:50051
ENV HTTP_SERVER_LOG_STYLE=json
EXPOSE 80

WORKDIR /app
ENTRYPOINT ["prometheus-grpc-proxy"]

# The test server image.
FROM debian:bullseye-slim as test-server

COPY --from=builder /usr/local/cargo/bin/server /usr/local/bin/test-server

ENV GRPC_SERVER_BIND_ENDPOINT=0.0.0.0:50051
EXPOSE 50051

WORKDIR /app
ENTRYPOINT ["test-server"]

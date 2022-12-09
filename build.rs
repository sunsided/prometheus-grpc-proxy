fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true) // for testing
        .build_client(true) // for the library
        .compile(
            &["protos/prometheus/prometheus.proto"],
            &["protos/prometheus"],
        )?;

    Ok(())
}

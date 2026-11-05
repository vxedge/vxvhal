fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Android's gRPC Vehicle HAL: vxvhal is the server, the client is for tests.
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/aosp/VehicleServer.proto"], &["proto/aosp"])?;
    // Eclipse Kuksa databroker API v2: vxvhal is a client.
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["proto/kuksa/kuksa/val/v2/val.proto"], &["proto/kuksa"])?;
    println!("cargo:rerun-if-changed=proto");
    Ok(())
}

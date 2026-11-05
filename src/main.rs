use std::path::PathBuf;

use clap::Parser;
use tonic::transport::Server;

use vxvhal::mapping::Mapping;
use vxvhal::pb::vhal::vehicle_server_server::VehicleServerServer;
use vxvhal::server::Vhal;
use vxvhal::signals::Signals;

/// Serve Android Automotive's gRPC Vehicle HAL from VSS signals in an Eclipse Kuksa databroker.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Address to serve the VehicleServer on.
    #[arg(long, env = "VXVHAL_LISTEN", default_value = "0.0.0.0:9300")]
    listen: std::net::SocketAddr,
    /// The Kuksa databroker (kuksa.val.v2).
    #[arg(
        long,
        env = "VXVHAL_DATABROKER",
        default_value = "http://127.0.0.1:55555"
    )]
    databroker: String,
    /// The VSS-to-AAOS mapping table.
    #[arg(long, env = "VXVHAL_MAPPING", default_value = "mappings/default.yaml")]
    mapping: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();
    let mapping = Mapping::load(&args.mapping)?;
    let signals = Signals::new(&args.databroker, &mapping.properties)?;
    tokio::spawn(signals.clone().run());
    tracing::info!(
        listen = %args.listen,
        databroker = %args.databroker,
        properties = mapping.properties.len(),
        "vxvhal serving"
    );
    Server::builder()
        .add_service(VehicleServerServer::new(Vhal::new(
            mapping.index(),
            signals,
        )))
        .serve_with_shutdown(args.listen, async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::Notify;
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
    let stop = Arc::new(Notify::new());
    let server = Server::builder()
        .add_service(VehicleServerServer::new(Vhal::new(
            mapping.index(),
            signals,
        )))
        .serve_with_shutdown(args.listen, {
            let stop = stop.clone();
            async move { stop.notified().await }
        });
    tokio::pin!(server);
    tokio::select! {
        served = &mut server => return served.map_err(Into::into),
        () = stopped() => {}
    }
    // Stop accepting, then give open calls a moment: CarService holds its property stream open for
    // as long as the guest runs, so a graceful shutdown alone would wait for the guest.
    tracing::info!("vxvhal stopping");
    stop.notify_one();
    match tokio::time::timeout(DRAIN, server).await {
        Ok(served) => served.map_err(Into::into),
        Err(_) => Ok(()),
    }
}

/// How long open calls may take to finish once vxvhal is asked to stop.
const DRAIN: Duration = Duration::from_secs(2);

/// Resolves on SIGINT or SIGTERM. A container runtime stops its entrypoint with SIGTERM, and as
/// PID 1 vxvhal gets no default action for it, so it must handle it to stop at all.
async fn stopped() {
    let mut term = signal(SignalKind::terminate()).expect("install the SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}

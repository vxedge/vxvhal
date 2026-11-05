//! vxvhal against a real Kuksa databroker: values published to VSS reach AAOS's gRPC VHAL API,
//! converted, through `GetValues` and the property stream.
//!
//! Needs a databroker: `scripts/databroker.sh` starts the pinned image and prints the address.
//! Runs only when `VXVHAL_TEST_DATABROKER` is set, so plain `cargo test` stays self-contained.

use std::time::Duration;

use tokio_stream::StreamExt;
use tonic::transport::{Channel, Server};

use vxvhal::mapping::Mapping;
use vxvhal::pb::kuksa::{self, val_client::ValClient};
use vxvhal::pb::vhal::{
    self, vehicle_server_client::VehicleServerClient, vehicle_server_server::VehicleServerServer,
};
use vxvhal::server::Vhal;
use vxvhal::signals::Signals;

const SPEED: i32 = 0x1160_0207;

fn databroker() -> Option<String> {
    std::env::var("VXVHAL_TEST_DATABROKER").ok()
}

async fn publish(kuksa: &mut ValClient<Channel>, path: &str, km_h: f32) {
    kuksa
        .publish_value(kuksa::PublishValueRequest {
            signal_id: Some(kuksa::SignalId {
                signal: Some(kuksa::signal_id::Signal::Path(path.into())),
            }),
            data_point: Some(kuksa::Datapoint {
                timestamp: None,
                value: Some(kuksa::Value {
                    typed_value: Some(kuksa::value::TypedValue::Float(km_h)),
                }),
            }),
        })
        .await
        .expect("publish to the databroker");
}

fn get(prop: i32) -> vhal::VehiclePropValueRequests {
    vhal::VehiclePropValueRequests {
        requests: vec![vhal::VehiclePropValueRequest {
            request_id: 1,
            value: Some(vhal::VehiclePropValue {
                prop,
                area_id: 0,
                ..Default::default()
            }),
        }],
    }
}

#[tokio::test]
async fn a_published_vss_speed_reaches_the_vhal_api_converted() {
    let Some(addr) = databroker() else {
        eprintln!("skipped: set VXVHAL_TEST_DATABROKER (scripts/databroker.sh)");
        return;
    };
    let mapping = Mapping::parse(include_str!("../mappings/default.yaml")).unwrap();
    let signals = Signals::new(&addr, &mapping.properties).unwrap();
    tokio::spawn(signals.clone().run());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local = listener.local_addr().unwrap();
    tokio::spawn(
        Server::builder()
            .add_service(VehicleServerServer::new(Vhal::new(
                mapping.index(),
                signals,
            )))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener)),
    );
    let mut vhal = VehicleServerClient::connect(format!("http://{local}"))
        .await
        .unwrap();
    let mut kuksa = ValClient::connect(addr).await.unwrap();

    // Every mapped property is announced.
    let configs: Vec<_> = vhal
        .get_all_property_config(())
        .await
        .unwrap()
        .into_inner()
        .collect::<Result<Vec<_>, _>>()
        .await
        .unwrap();
    assert_eq!(configs.len(), mapping.properties.len());
    assert!(configs.iter().any(|c| c.prop == SPEED));

    // A value already in the databroker is read through GetValues: 72 km/h is 20 m/s.
    publish(&mut kuksa, "Vehicle.Speed", 72.0).await;
    let got = vhal.get_values(get(SPEED)).await.unwrap().into_inner();
    let r = &got.results[0];
    assert_eq!(r.status, vhal::StatusCode::Ok as i32, "{got:?}");
    let v = r.value.as_ref().unwrap();
    assert!((v.float_values[0] - 20.0).abs() < 1e-3, "{v:?}");

    // A later value arrives on the property stream: 36 km/h is 10 m/s.
    let mut stream = vhal
        .start_property_values_stream(())
        .await
        .unwrap()
        .into_inner();
    // Give the subscription a moment to be in place, then publish until the stream carries it.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        publish(&mut kuksa, "Vehicle.Speed", 36.0).await;
        match tokio::time::timeout(Duration::from_millis(500), stream.next()).await {
            Ok(Some(Ok(batch))) => {
                if let Some(v) = batch.values.iter().find(|v| v.prop == SPEED) {
                    assert!((v.float_values[0] - 10.0).abs() < 1e-3, "{v:?}");
                    assert!(v.timestamp > 0);
                    break;
                }
            }
            Ok(other) => panic!("stream ended: {other:?}"),
            Err(_) => assert!(
                tokio::time::Instant::now() < deadline,
                "no update on the property stream within 10 s"
            ),
        }
    }

    // An unmapped property is an invalid argument, and writes are refused in 0.1.
    let bad = vhal
        .get_values(get(0x1160_0999))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(bad.results[0].status, vhal::StatusCode::InvalidArg as i32);
    let mut set = get(SPEED);
    set.requests[0].value.as_mut().unwrap().float_values = vec![1.0];
    let denied = vhal.set_values(set).await.unwrap().into_inner();
    assert_eq!(
        denied.results[0].status,
        vhal::StatusCode::AccessDenied as i32
    );
}

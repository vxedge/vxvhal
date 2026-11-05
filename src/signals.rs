//! The Kuksa side: current values on demand, and a stream of AAOS property updates.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::broadcast;
use tonic::transport::{Channel, Endpoint};

use crate::mapping::Property;
use crate::pb::kuksa::{self, val_client::ValClient};
use crate::pb::vhal;

/// How long to wait before resubscribing after the databroker drops or refuses the stream.
const RESUBSCRIBE_AFTER: Duration = Duration::from_secs(1);

/// The databroker connection and the update fan-out shared by every gRPC call.
#[derive(Clone)]
pub struct Signals {
    client: ValClient<Channel>,
    by_path: Arc<HashMap<String, Vec<Property>>>,
    updates: broadcast::Sender<vhal::VehiclePropValues>,
    /// The last timestamp sent per (prop, area). Android's proxy drops a value whose timestamp
    /// is older than the last one it saw, so timestamps must only move forward.
    last_ts: Arc<Mutex<HashMap<(i32, i32), i64>>>,
}

/// Why the databroker could not answer.
#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    #[error("invalid databroker address {0}")]
    Address(String),
    #[error("databroker call failed: {0}")]
    Call(#[from] tonic::Status),
}

impl Signals {
    /// Connects lazily: vxvhal serves (reporting values not available) while the databroker is
    /// still starting, and picks it up when it answers.
    pub fn new(databroker: &str, properties: &[Property]) -> Result<Self, SignalError> {
        let channel = Endpoint::from_shared(databroker.to_string())
            .map_err(|_| SignalError::Address(databroker.to_string()))?
            .connect_timeout(Duration::from_secs(2))
            .connect_lazy();
        let mut by_path: HashMap<String, Vec<Property>> = HashMap::new();
        for p in properties {
            by_path.entry(p.vss.clone()).or_default().push(p.clone());
        }
        let (updates, _) = broadcast::channel(1024);
        Ok(Self {
            client: ValClient::new(channel),
            by_path: Arc::new(by_path),
            updates,
            last_ts: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Updates for every mapped property, as they arrive from the databroker.
    pub fn subscribe(&self) -> broadcast::Receiver<vhal::VehiclePropValues> {
        self.updates.subscribe()
    }

    /// The current value of `property`, read from the databroker now. `Ok(None)` when the
    /// signal has no value yet or one that does not convert.
    pub async fn get(
        &self,
        property: &Property,
    ) -> Result<Option<vhal::VehiclePropValue>, SignalError> {
        let request = kuksa::GetValuesRequest {
            signal_ids: vec![kuksa::SignalId {
                signal: Some(kuksa::signal_id::Signal::Path(property.vss.clone())),
            }],
        };
        let response = self.client.clone().get_values(request).await?.into_inner();
        Ok(response
            .data_points
            .first()
            .and_then(|dp| self.to_value(property, dp)))
    }

    /// Whether the databroker answers.
    pub async fn healthy(&self) -> bool {
        self.client
            .clone()
            .get_server_info(kuksa::GetServerInfoRequest {})
            .await
            .is_ok()
    }

    /// Follows the databroker until the process ends, resubscribing after any failure.
    pub async fn run(self) {
        let paths: Vec<String> = self.by_path.keys().cloned().collect();
        loop {
            let request = kuksa::SubscribeRequest {
                signal_paths: paths.clone(),
                buffer_size: 0,
                filter: None,
            };
            match self.client.clone().subscribe(request).await {
                Ok(response) => {
                    tracing::info!(signals = paths.len(), "subscribed to the databroker");
                    let mut stream = response.into_inner();
                    loop {
                        match stream.message().await {
                            Ok(Some(update)) => self.publish(update),
                            Ok(None) => {
                                tracing::warn!("databroker ended the subscription");
                                break;
                            }
                            Err(status) => {
                                tracing::warn!(%status, "databroker subscription failed");
                                break;
                            }
                        }
                    }
                }
                Err(status) => {
                    tracing::warn!(%status, ?paths, "cannot subscribe to the databroker")
                }
            }
            tokio::time::sleep(RESUBSCRIBE_AFTER).await;
        }
    }

    fn publish(&self, update: kuksa::SubscribeResponse) {
        let mut values = Vec::new();
        for (path, dp) in &update.entries {
            for property in self.by_path.get(path).into_iter().flatten() {
                if let Some(v) = self.to_value(property, dp) {
                    values.push(v);
                }
            }
        }
        if !values.is_empty() {
            // No receiver means no AAOS stream is open yet; nothing is lost that `get` cannot read.
            let _ = self.updates.send(vhal::VehiclePropValues { values });
        }
    }

    fn to_value(
        &self,
        property: &Property,
        dp: &kuksa::Datapoint,
    ) -> Option<vhal::VehiclePropValue> {
        let typed = dp.value.as_ref()?.typed_value.as_ref()?;
        let ts = self.next_timestamp(property, dp.timestamp.as_ref());
        property.convert(typed, ts)
    }

    fn next_timestamp(&self, property: &Property, ts: Option<&prost_types::Timestamp>) -> i64 {
        let ns = ts
            .map(|t| {
                t.seconds
                    .saturating_mul(1_000_000_000)
                    .saturating_add(i64::from(t.nanos))
            })
            .unwrap_or_else(now_ns);
        let mut last = self.last_ts.lock().expect("timestamp map lock");
        let entry = last.entry((property.prop, property.area)).or_insert(0);
        // The same datapoint read twice keeps its timestamp; anything older moves forward.
        let out = if ns >= *entry { ns } else { *entry };
        *entry = out;
        out
    }
}

fn now_ns() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

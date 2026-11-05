//! The `VehicleServer` gRPC service that AAOS's `GRPCVehicleHardware` calls.

use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};

use crate::mapping::Property;
use crate::pb::vhal::{self, vehicle_server_server::VehicleServer};
use crate::signals::Signals;

type Stream_<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + 'static>>;

/// The service: a read-only Vehicle HAL backed by [`Signals`].
pub struct Vhal {
    properties: Arc<HashMap<(i32, i32), Property>>,
    signals: Signals,
    /// What AAOS has subscribed to, for `Dump`. Updates go to every open stream regardless:
    /// AAOS's DefaultVehicleHal filters by its own subscriptions.
    subscribed: Mutex<HashSet<(i32, i32)>>,
}

impl Vhal {
    pub fn new(properties: HashMap<(i32, i32), Property>, signals: Signals) -> Self {
        Self {
            properties: Arc::new(properties),
            signals,
            subscribed: Mutex::new(HashSet::new()),
        }
    }

    fn property(&self, prop: i32, area: i32) -> Option<&Property> {
        self.properties.get(&(prop, area))
    }
}

fn status(code: vhal::StatusCode) -> vhal::VehicleHalCallStatus {
    vhal::VehicleHalCallStatus {
        status_code: code as i32,
    }
}

#[tonic::async_trait]
impl VehicleServer for Vhal {
    type GetAllPropertyConfigStream = Stream_<vhal::VehiclePropConfig>;

    async fn get_all_property_config(
        &self,
        _: Request<()>,
    ) -> Result<Response<Self::GetAllPropertyConfigStream>, Status> {
        let mut configs: Vec<_> = self.properties.values().map(Property::config).collect();
        configs.sort_by_key(|c| c.prop);
        Ok(Response::new(Box::pin(tokio_stream::iter(
            configs.into_iter().map(Ok),
        ))))
    }

    async fn set_values(
        &self,
        request: Request<vhal::VehiclePropValueRequests>,
    ) -> Result<Response<vhal::SetValueResults>, Status> {
        // Read-only in 0.1: a mapped property refuses writes, an unmapped one is invalid.
        let results = request
            .into_inner()
            .requests
            .into_iter()
            .map(|r| {
                let code = match r.value {
                    Some(v) if self.property(v.prop, v.area_id).is_some() => {
                        vhal::StatusCode::AccessDenied
                    }
                    _ => vhal::StatusCode::InvalidArg,
                };
                vhal::SetValueResult {
                    request_id: r.request_id,
                    status: code as i32,
                }
            })
            .collect();
        Ok(Response::new(vhal::SetValueResults { results }))
    }

    async fn get_values(
        &self,
        request: Request<vhal::VehiclePropValueRequests>,
    ) -> Result<Response<vhal::GetValueResults>, Status> {
        let mut results = Vec::new();
        for r in request.into_inner().requests {
            let Some(asked) = r.value else {
                results.push(result(r.request_id, vhal::StatusCode::InvalidArg, None));
                continue;
            };
            let Some(property) = self.property(asked.prop, asked.area_id) else {
                results.push(result(r.request_id, vhal::StatusCode::InvalidArg, None));
                continue;
            };
            results.push(match self.signals.get(property).await {
                Ok(Some(value)) => result(r.request_id, vhal::StatusCode::Ok, Some(value)),
                Ok(None) => result(r.request_id, vhal::StatusCode::NotAvailable, None),
                Err(e) => {
                    tracing::warn!(error = %e, property = %property.name, "get failed");
                    result(r.request_id, vhal::StatusCode::TryAgain, None)
                }
            });
        }
        Ok(Response::new(vhal::GetValueResults { results }))
    }

    async fn update_sample_rate(
        &self,
        request: Request<vhal::UpdateSampleRateRequest>,
    ) -> Result<Response<vhal::VehicleHalCallStatus>, Status> {
        let r = request.into_inner();
        // Updates are pushed as the databroker sends them; the rate is accepted, not enforced.
        Ok(Response::new(status(
            match self.property(r.prop, r.area_id) {
                Some(_) => vhal::StatusCode::Ok,
                None => vhal::StatusCode::InvalidArg,
            },
        )))
    }

    async fn check_health(
        &self,
        _: Request<()>,
    ) -> Result<Response<vhal::VehicleHalCallStatus>, Status> {
        Ok(Response::new(status(if self.signals.healthy().await {
            vhal::StatusCode::Ok
        } else {
            vhal::StatusCode::TryAgain
        })))
    }

    async fn dump(
        &self,
        _: Request<vhal::DumpOptions>,
    ) -> Result<Response<vhal::DumpResult>, Status> {
        let subscribed = self.subscribed.lock().expect("subscription lock").clone();
        let mut lines: Vec<String> = self
            .properties
            .values()
            .map(|p| {
                let sub = if subscribed.contains(&(p.prop, p.area)) {
                    " subscribed"
                } else {
                    ""
                };
                format!("{} {:#x} area {} <- {}{sub}", p.name, p.prop, p.area, p.vss)
            })
            .collect();
        lines.sort();
        Ok(Response::new(vhal::DumpResult {
            caller_should_dump_state: true,
            buffer: format!(
                "vxvhal {}\n{}\n",
                env!("CARGO_PKG_VERSION"),
                lines.join("\n")
            ),
            refresh_property_configs: false,
        }))
    }

    type StartPropertyValuesStreamStream = Stream_<vhal::VehiclePropValues>;

    async fn start_property_values_stream(
        &self,
        _: Request<()>,
    ) -> Result<Response<Self::StartPropertyValuesStreamStream>, Status> {
        // A slow reader that falls behind skips the values it missed; the next ones still flow.
        let stream = BroadcastStream::new(self.signals.subscribe()).filter_map(|r| r.ok().map(Ok));
        Ok(Response::new(Box::pin(stream)))
    }

    async fn subscribe(
        &self,
        request: Request<vhal::SubscribeRequest>,
    ) -> Result<Response<vhal::VehicleHalCallStatus>, Status> {
        let Some(options) = request.into_inner().options else {
            return Ok(Response::new(status(vhal::StatusCode::InvalidArg)));
        };
        let areas = if options.area_ids.is_empty() {
            vec![0]
        } else {
            options.area_ids
        };
        if areas
            .iter()
            .any(|a| self.property(options.prop_id, *a).is_none())
        {
            return Ok(Response::new(status(vhal::StatusCode::InvalidArg)));
        }
        let mut subscribed = self.subscribed.lock().expect("subscription lock");
        subscribed.extend(areas.into_iter().map(|a| (options.prop_id, a)));
        Ok(Response::new(status(vhal::StatusCode::Ok)))
    }

    async fn unsubscribe(
        &self,
        request: Request<vhal::UnsubscribeRequest>,
    ) -> Result<Response<vhal::VehicleHalCallStatus>, Status> {
        let r = request.into_inner();
        self.subscribed
            .lock()
            .expect("subscription lock")
            .remove(&(r.prop_id, r.area_id));
        Ok(Response::new(status(vhal::StatusCode::Ok)))
    }

    async fn get_min_max_supported_values(
        &self,
        _: Request<vhal::GetMinMaxSupportedValuesRequest>,
    ) -> Result<Response<vhal::GetMinMaxSupportedValuesResult>, Status> {
        Err(Status::unimplemented(
            "no mapped property declares supported-value info",
        ))
    }

    async fn get_supported_values_lists(
        &self,
        _: Request<vhal::GetSupportedValuesListsRequest>,
    ) -> Result<Response<vhal::GetSupportedValuesListsResult>, Status> {
        Err(Status::unimplemented(
            "no mapped property declares supported-value info",
        ))
    }

    type StartSupportedValuesChangeStreamStream = Stream_<vhal::SupportedValuesChange>;

    async fn start_supported_values_change_stream(
        &self,
        _: Request<()>,
    ) -> Result<Response<Self::StartSupportedValuesChangeStreamStream>, Status> {
        // Nothing changes: an empty stream that stays open is what a caller expects.
        Ok(Response::new(Box::pin(tokio_stream::pending())))
    }
}

fn result(
    request_id: i64,
    code: vhal::StatusCode,
    value: Option<vhal::VehiclePropValue>,
) -> vhal::GetValueResult {
    vhal::GetValueResult {
        request_id,
        status: code as i32,
        value,
    }
}

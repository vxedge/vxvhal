//! vxvhal: an Eclipse Kuksa provider that serves Android Automotive's gRPC Vehicle HAL.
//!
//! AAOS's `GRPCVehicleHardware` forwards every Vehicle HAL call to a gRPC `VehicleServer`. vxvhal
//! is that server: it answers from the VSS signals in a Kuksa databroker, through a mapping table
//! of VSS path to AAOS property ID, area and unit conversion.

pub mod mapping;
pub mod server;
pub mod signals;

/// Generated Protobuf and gRPC code. Lints are allowed here only: the doc comments come from
/// the upstream `.proto` files.
#[allow(clippy::all, clippy::doc_lazy_continuation)]
pub mod pb {
    /// Android's gRPC Vehicle HAL (`android.hardware.automotive.vehicle.proto`).
    pub mod vhal {
        tonic::include_proto!("android.hardware.automotive.vehicle.proto");
    }
    /// The Kuksa databroker API (`kuksa.val.v2`).
    pub mod kuksa {
        tonic::include_proto!("kuksa.val.v2");
    }
}

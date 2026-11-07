# vxvhal

An [Eclipse Kuksa](https://github.com/eclipse-kuksa) provider that serves Android Automotive's gRPC Vehicle
HAL from [COVESA VSS](https://github.com/COVESA/vehicle_signal_specification) signals. It gives an AAOS guest in
a vxedge virtual prototype trial the same vehicle-data path a production car has: Android's Vehicle HAL, backed
by a gateway that speaks the vehicle network.

```
AAOS guest                         vxvhal                          Kuksa databroker
CarService -> DefaultVehicleHal    VehicleServer (gRPC)            kuksa.val.v2
           -> GRPCVehicleHardware ─────────────────> mapping ─────> VSS signals <── zone ECUs, CAN, ...
```

Since Android 16, AOSP ships `GRPCVehicleHardware`, a Vehicle HAL that forwards every call to a `VehicleServer`
on another machine or VM. vxvhal is that server. It reads values from a Kuksa databroker, converts each one
through a mapping table (VSS path to AAOS property ID, area and unit), and streams updates as the databroker
sends them.

## Status: 0.1, read path

| Call | 0.1 |
|---|---|
| `GetAllPropertyConfig` | Every mapped property, read-only |
| `GetValues` | Reads the databroker now and converts; `NOT_AVAILABLE` until the signal has a value |
| `StartPropertyValuesStream` | Every update from the databroker, converted, with timestamps that only move forward |
| `Subscribe`, `Unsubscribe`, `UpdateSampleRate` | Accepted and recorded; updates are pushed as they arrive, the rate is not enforced |
| `CheckHealth` | `OK` when the databroker answers |
| `SetValues` | `ACCESS_DENIED`: writes to actuators (Kuksa `Actuate`) come in 0.2 |
| Supported-value calls | Not used: no mapped property declares supported-value info |

The default mapping (`mappings/default.yaml`) carries four properties: `PERF_VEHICLE_SPEED` from
`Vehicle.Speed` (km/h to m/s), `ENGINE_RPM` from `Vehicle.Powertrain.CombustionEngine.Speed`, and
`PERF_ODOMETER` from `Vehicle.TraveledDistance`, and `GEAR_SELECTION` from
`Vehicle.Powertrain.Transmission.SelectedGear`, an enum mapped through the row's `values` table.

## What it is not

- Not a vendor Vehicle HAL, and not tested with CTS or VTS.
- Run against one AAOS guest so far: Cuttlefish with AAOS 26Q2 (Android 17, API 37). Values reached CarService
  both on request and over the property stream (`docs/aaos-cuttlefish.md`).
- The mapping is hand-written. COVESA's AASIG binding and its `vspec2aaproperties` generator are the source of
  the pairs, but COVESA describes that mapping as needing an update, and it is one-way.

## Run

```
cargo run --release -- --databroker http://127.0.0.1:55555 --mapping mappings/default.yaml --listen 0.0.0.0:9300
```

In the AAOS guest, the Vehicle HAL must be the gRPC one, pointed at this address. Cuttlefish's auto guest
already is: `scripts/cuttlefish-vhal-proxy.sh` replaces Cuttlefish's host `vhal_proxy_server` with vxvhal
(`docs/aaos-cuttlefish.md`).

## Test

```
cargo test                                   # unit tests
export VXVHAL_TEST_DATABROKER=$(scripts/databroker.sh)
cargo test --test databroker                 # against the pinned Kuksa databroker 0.7.1
scripts/databroker.sh stop
```

The integration test publishes `Vehicle.Speed` to the databroker. It checks that 72 km/h reads back through
`GetValues` as 20 m/s, and that a later 36 km/h arrives on the property stream as 10 m/s.

## Interfaces

Both APIs are vendored, pinned and unmodified (`proto/PROVENANCE.md`): AOSP's `VehicleServer.proto` and Vehicle
HAL types from `android17-release` (AAOS 26Q2, API level 37), and Kuksa's `kuksa.val.v2` from the databroker 0.7.1
release.

## Licence

Apache-2.0. See `LICENSE` and `NOTICE`.

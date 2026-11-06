# Changelog

All notable changes to this repo are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- A `VehicleServer` for AAOS's `GRPCVehicleHardware`, backed by a Kuksa databroker (`kuksa.val.v2`):
  property configs, `GetValues`, the property value stream, subscription bookkeeping and a health check.
  Read-only: `SetValues` answers `ACCESS_DENIED`.
- A YAML mapping of VSS path to AAOS property ID, area and unit conversion, with three properties
  (`PERF_VEHICLE_SPEED`, `ENGINE_RPM`, `PERF_ODOMETER`).
- An integration test against the Kuksa databroker 0.7.1 image that vxparts pins.
- `scripts/cuttlefish-vhal-proxy.sh`, a drop-in for Cuttlefish's host `vhal_proxy_server`, and
  `docs/aaos-cuttlefish.md`: AAOS 26Q2 on Cuttlefish read `PERF_VEHICLE_SPEED` and `ENGINE_RPM` from Kuksa
  through vxvhal, on request and over the property stream into CarService.
- Vendored, pinned interfaces: AOSP `android17-release` (AAOS 26Q2) and `kuksa-proto` from databroker 0.7.1.

# Changelog

All notable changes to this repo are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.1] - 2026-11-08

### Fixed

- vxvhal stops on SIGTERM. As a container's entrypoint it runs as PID 1, which gets no default action for
  SIGTERM, so a container stop waited out the runtime's timeout and then killed it. It now stops accepting
  calls and exits after at most 2 s, without waiting for CarService's property stream, which stays open for as
  long as the guest runs.

## [0.1.0] - 2026-11-08

First release: the read path from a Kuksa databroker to an AAOS 26Q2 guest's Vehicle HAL.

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
- Enum properties: a mapping row may give `values` (VSS value to AAOS value) and a `config_array`. The
  default mapping adds `GEAR_SELECTION` from `Vehicle.Powertrain.Transmission.SelectedGear` (neutral,
  reverse, park, drive), which AAOS's EVS app needs to start.
- Vendored, pinned interfaces: AOSP `android17-release` (AAOS 26Q2) and `kuksa-proto` from databroker 0.7.1.
- `Containerfile`: vxvhal as a container payload, a static musl binary on `scratch` with the default
  mapping, listening on `:9300` and reading the databroker at `127.0.0.1:55555`; the image the vxparts part
  `svc/signal/app/vxvhal` publishes.

[Unreleased]: https://github.com/vxedge/vxvhal/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/vxedge/vxvhal/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/vxedge/vxvhal/releases/tag/v0.1.0

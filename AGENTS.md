# AGENTS.md — vxedge/vxvhal

An Eclipse Kuksa provider that serves Android Automotive's gRPC Vehicle HAL (`VehicleServer`) from VSS
signals. Rust: tonic 0.14 + tonic-prost, prost 0.14, edition 2024, `serde_yaml_ng`.

## Rules

- `proto/` is vendored and unmodified. Change it only by re-fetching at a new pin and updating
  `proto/PROVENANCE.md` in the same change.
- Every mapping row cites its source: the property ID from `VehicleProperty.aidl` at the pinned AOSP commit,
  and the VSS path from the databroker's VSS release. Units convert as `value * scale + offset`.
- Timestamps sent on the stream only move forward per property and area: Android's proxy drops older ones.
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` and `cargo deny check`
  pass before every push. The databroker test runs with `VXVHAL_TEST_DATABROKER=$(scripts/databroker.sh)`.

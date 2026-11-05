# Vendored interfaces

Copied unmodified. Each file keeps its own licence header (Apache-2.0).

| Directory | Source | Pin |
|---|---|---|
| `aosp/VehicleServer.proto` | `platform/hardware/interfaces`, `automotive/vehicle/aidl/impl/current/grpc/proto/` | `0162af698935100a590b7359581ac8b1b80693e5` (`android17-release`, fetched 2026-11-06) |
| `aosp/android/hardware/automotive/vehicle/` | `platform/hardware/interfaces`, `automotive/vehicle/aidl/impl/current/proto/android/hardware/automotive/vehicle/` | same commit |
| `kuksa/kuksa/val/v2/` | `eclipse-kuksa/kuksa-proto`, `kuksa/val/v2/` | `28bf76ae57085dad425eedd95bc7bd442e1314a7`, the `proto` submodule of `eclipse-kuksa/kuksa-databroker` tag `0.7.1` |

AAOS 26Q2 is API level 37, the Android 17 release; `android17-release` is the AOSP branch of that release.
The property IDs in `../mappings/default.yaml` come from `automotive/vehicle/aidl_property/android/hardware/automotive/vehicle/VehicleProperty.aidl`
at the same AOSP commit.

To update: fetch the same paths at the new pin, replace the files, update this table, and run the tests.

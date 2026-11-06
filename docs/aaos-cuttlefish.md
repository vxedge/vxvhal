# vxvhal with AAOS on Cuttlefish

Cuttlefish's auto guest already uses `GRPCVehicleHardware`: its Vehicle HAL dials a `VehicleServer` on the host,
`192.168.98.1:<port>` (port 9300 for the first instance), which `run_cvd` starts as the host binary
`vhal_proxy_server` when launched with `--enable_vhal_proxy_server=true`. Upstream, that server runs AOSP's
`FakeVehicleHardware`. `scripts/cuttlefish-vhal-proxy.sh` takes its place and runs vxvhal, so the guest reads
VSS signals from a Kuksa databroker. No Android image is rebuilt.

## Run, 2026-11-06

| Item | Value |
|---|---|
| AAOS | Cuttlefish `aosp_cf_x86_64_auto-userdebug`, build 16644040 from `aosp-android-latest-release`; in the guest: Android 17, API level 37 (AAOS 26Q2), `ro.build.id` `CP2A.260605.016` |
| Host tools | `cuttlefish-base` 1.57.13, the build's own host package |
| Launch | `launch_cvd --daemon --enable_vhal_proxy_server=true --start_webrtc=false --gpu_mode=guest_swiftshader --cpus=4 --memory_mb=6144` |
| vxvhal | 0.1.0, `mappings/default.yaml`, serving `192.168.98.1:9300` |
| Databroker | Kuksa 0.7.1, the image `scripts/databroker.sh` pins |

The guest's Vehicle HAL took its property configs from vxvhal: it refused subscriptions to properties vxvhal does
not map, such as `GEAR_SELECTION`. Values were published with the Kuksa CLI and read in the guest with
`cmd car_service get-property-value`:

| Published to Kuksa | Read in AAOS |
|---|---|
| `Vehicle.Speed` 72.0 | `PERF_VEHICLE_SPEED(0x11600207)` `AVAILABLE` 20.0 m/s |
| `Vehicle.Speed` 36.0 | `PERF_VEHICLE_SPEED(0x11600207)` `AVAILABLE` 10.0 m/s |
| `Vehicle.Powertrain.CombustionEngine.Speed` 2500 | `ENGINE_RPM(0x11600305)` `AVAILABLE` 2500.0 rpm |

## Limits

- One host, one guest, through `GetValues`. CarService subscriptions over the property stream are served but were
  not checked in the guest in this run.
- vxvhal serves the guest's Ethernet path only. The vsock address `run_cvd` also passes is logged and not served.
- The Cuttlefish network here is Cuttlefish's own bridge (`cvd-ebr`), not a vxedge slice.

## Host notes

- On Ubuntu 26.04, unprivileged user namespaces are restricted by AppArmor, and crosvm's sandbox fails with
  `unshare(CLONE_NEWNS) failed: Operation not permitted`. An AppArmor profile that grants `userns` to
  `/usr/lib/cuttlefish-common/bin/crosvm` alone fixes it, and the sandbox stays on.
- The Kuksa CLI 0.7.1 needs a terminal: run it with `podman run -t` under `script` when scripting.

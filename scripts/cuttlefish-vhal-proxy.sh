#!/usr/bin/env bash
# A drop-in for Cuttlefish's host `vhal_proxy_server` that runs vxvhal instead of AOSP's
# FakeVehicleHardware. Cuttlefish's auto guest uses GRPCVehicleHardware and dials this server.
#
# Install into a Cuttlefish host package:
#   mv <cf>/bin/vhal_proxy_server <cf>/bin/vhal_proxy_server.aosp
#   cp scripts/cuttlefish-vhal-proxy.sh <cf>/bin/vhal_proxy_server
# then launch with --enable_vhal_proxy_server=true.
#
# run_cvd calls it as: vhal_proxy_server <config-dir> <eth-ip:port> <vsock address>
# (device/google/cuttlefish host/commands/run_cvd/launch/vhal_proxy_server.cpp). The guest dials the
# Ethernet address once its auto-ethernet namespace is up, so vxvhal serves that one; the vsock
# address is logged and not served.
#
# Environment: VXVHAL_BIN (default: vxvhal on PATH), VXVHAL_DATABROKER, VXVHAL_MAPPING.
set -euo pipefail

config_dir=${1:?config dir}
eth_addr=${2:?eth address}
vsock_addr=${3:-}

echo "vxvhal shim: config $config_dir ignored; serving $eth_addr; not serving $vsock_addr" >&2
exec "${VXVHAL_BIN:-vxvhal}" \
    --listen "$eth_addr" \
    --databroker "${VXVHAL_DATABROKER:-http://127.0.0.1:55555}" \
    --mapping "${VXVHAL_MAPPING:?set VXVHAL_MAPPING to a mapping file}"

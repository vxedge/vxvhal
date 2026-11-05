#!/usr/bin/env bash
# Start the Kuksa databroker that vxparts pins (svc/signal/app/kuksa-databroker), for the
# integration test, and print the address to export as VXVHAL_TEST_DATABROKER.
#   ./scripts/databroker.sh          start (idempotent)
#   ./scripts/databroker.sh stop     stop and remove
set -euo pipefail

image=ghcr.io/eclipse-kuksa/kuksa-databroker@sha256:529d8c6d3cc223362667672978bcb8f92c4d356b25aedd336e7a2e004d7279bd
name=vxvhal-test-databroker
port=${VXVHAL_TEST_PORT:-55556}

if [ "${1:-}" = stop ]; then
    podman rm -f "$name" >/dev/null 2>&1 || true
    exit 0
fi
if ! podman container exists "$name"; then
    podman run -d --rm --name "$name" -p "127.0.0.1:$port:55555" "$image" --insecure >/dev/null
fi
# Wait until it accepts connections, so a test can connect at once.
for _ in $(seq 60); do
    (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null && break
    sleep 0.5
done
echo "http://127.0.0.1:$port"

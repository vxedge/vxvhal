# vxvhal as a container payload: the static musl build on an empty base, with the default mapping.
# Built from a checkout:
#   cargo build --release --target x86_64-unknown-linux-musl
#   podman build -t vxvhal -f Containerfile .
# The databroker defaults to the same node (127.0.0.1:55555): place the vehicle's signal plane on the
# node that runs vxvhal, as a gateway ECU hosts both, or override VXVHAL_DATABROKER.
FROM scratch
COPY target/x86_64-unknown-linux-musl/release/vxvhal /vxvhal
COPY mappings/default.yaml /etc/vxvhal/mapping.yaml
ENV VXVHAL_LISTEN=0.0.0.0:9300 \
    VXVHAL_DATABROKER=http://127.0.0.1:55555 \
    VXVHAL_MAPPING=/etc/vxvhal/mapping.yaml
ENTRYPOINT ["/vxvhal"]

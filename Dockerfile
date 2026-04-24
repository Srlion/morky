# Stage 1: Build frontend with Deno
FROM denoland/deno:2.7.12 AS frontend
WORKDIR /src/frontend
COPY frontend/deno.json frontend/deno.lock frontend/package.json frontend/svelte.config.js frontend/vite.config.js ./
RUN deno install --node-modules-dir
COPY frontend/ .
RUN deno task build

# Stage 2: Build Rust binary
FROM docker.io/library/rust:alpine3.22 AS builder
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static && \
    rustup target add x86_64-unknown-linux-musl
WORKDIR /src
ARG CARGO_PROFILE=release

COPY . .
COPY --from=frontend /src/frontend/dist /src/frontend/dist

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --target x86_64-unknown-linux-musl --profile "$CARGO_PROFILE" && \
    PROFILE_DIR=$(if [ "$CARGO_PROFILE" = "dev" ] || [ "$CARGO_PROFILE" = "debug" ]; then echo "debug"; else echo "$CARGO_PROFILE"; fi) && \
    cp /src/target/x86_64-unknown-linux-musl/$PROFILE_DIR/morky /usr/local/bin/morky

# Stage 3: Runtime
FROM docker.io/library/debian:trixie-20260406-slim AS runtime-base
ARG RUNC_VERSION=v1.4.2
ARG PODMAN_VERSION=v5.8.2
ARG BUILDKIT_VERSION=v0.29.0
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl unzip git \
    && curl -fsSL -o /usr/local/bin/runc \
            https://github.com/opencontainers/runc/releases/download/${RUNC_VERSION}/runc.amd64 \
    && chmod +x /usr/local/bin/runc \
    && curl -fsSL https://github.com/containers/podman/releases/download/${PODMAN_VERSION}/podman-remote-static-linux_amd64.tar.gz \
        | tar xz -C /tmp \
    && mv /tmp/bin/podman-remote-static-linux_amd64 /usr/local/bin/podman \
    && chmod +x /usr/local/bin/podman \
    && curl -fsSL https://github.com/moby/buildkit/releases/download/${BUILDKIT_VERSION}/buildkit-${BUILDKIT_VERSION}.linux-amd64.tar.gz \
        | tar xz -C /tmp \
    && cp /tmp/bin/buildctl /usr/local/bin/ \
    && curl -fsSL https://railpack.com/install.sh | bash -s -- --bin-dir /usr/local/bin --yes \
    && rm -rf /tmp/* /var/lib/apt/lists/*

FROM runtime-base
COPY --from=builder /usr/local/bin/morky /usr/local/bin/morky
ENV MORKY_DATA_DIR=/data
ENV PODMAN_SOCKET=/run/podman/podman.sock
ENTRYPOINT ["morky"]

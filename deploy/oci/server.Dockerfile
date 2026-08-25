# syntax=docker/dockerfile:1.7

FROM rust:1.91.0-bookworm AS build

WORKDIR /workspace

# Copy only manifests and the selected desktop workspace manifest first. The
# desktop crate is never compiled, but Cargo needs every workspace member to be
# structurally present while resolving the locked dependency graph.
COPY Cargo.toml Cargo.lock ./
COPY apps/server/Cargo.toml apps/server/Cargo.toml
COPY apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/Cargo.toml
COPY apps/desktop/src-tauri/build.rs apps/desktop/src-tauri/build.rs
COPY apps/desktop/src-tauri/src apps/desktop/src-tauri/src

COPY apps/server/src apps/server/src

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    cargo build --locked --release --package ai-center-server

FROM debian:12.11-slim AS runtime

ARG OCI_VERSION=dev
ARG OCI_REVISION=unknown

LABEL org.opencontainers.image.title="AI Center API" \
      org.opencontainers.image.description="Context-control API for AI Center" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.source="https://github.com/gneed49/ai-center" \
      org.opencontainers.image.licenses="UNLICENSED"

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl tini \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 ai-center \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin ai-center

COPY --from=build --chown=10001:10001 \
    /workspace/target/release/ai-center-server \
    /usr/local/bin/ai-center-server

# Binding is the only safe container default. The server still requires an
# explicit production auth configuration before it will accept this non-loopback
# address, and all credentials remain runtime-only environment variables or
# mounted secret files.
ENV AI_CENTER_BIND=0.0.0.0:4317 \
    RUST_LOG=ai_center_server=info,tower_http=info

USER 10001:10001

EXPOSE 4317

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD ["curl", "--fail", "--silent", "--show-error", "--max-time", "3", "http://127.0.0.1:4317/api/health"]

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/bin/ai-center-server"]

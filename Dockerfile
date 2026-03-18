# syntax=docker/dockerfile:1
# ------------------------------------------------------------------------------
#  Subtensor Dockerfile (hardened)
#  – Builds production and local binaries
#  – Final runtime images run as non-root `subtensor` user (UID/GID 10001)
#
#  Incremental rebuilds: use BuildKit (DOCKER_BUILDKIT=1). Cargo registry, git
#  sources, and target/ are persisted across builds via cache mounts — small
#  source edits reuse compiled deps. Binaries are copied into /artifacts in the
#  same RUN because cache mounts are not part of the image layer.
# ------------------------------------------------------------------------------

###############################################################################
# ---------- 1. Common build environment -------------------------------------
###############################################################################
ARG BASE_IMAGE=rust:latest
FROM ${BASE_IMAGE} AS base_builder

LABEL ai.opentensor.image.authors="operations@opentensor.ai" \
  ai.opentensor.image.vendor="Opentensor Foundation" \
  ai.opentensor.image.title="opentensor/subtensor" \
  ai.opentensor.image.description="Opentensor Subtensor Blockchain" \
  ai.opentensor.image.documentation="https://docs.bittensor.com"

# Rust targets
RUN rustup update stable && \
  rustup target add wasm32-unknown-unknown --toolchain stable

# Build prerequisites
ENV RUST_BACKTRACE=1
RUN apt-get update && \
  apt-get install -y --no-install-recommends \
  curl build-essential protobuf-compiler clang git pkg-config libssl-dev && \
  rm -rf /var/lib/apt/lists/*

# Copy entire repository once for all build stages (maximises cache hits)
COPY . /build
WORKDIR /build

###############################################################################
# ---------- 2. Production build stage ---------------------------------------
###############################################################################
FROM base_builder AS prod_builder

RUN mkdir -p /artifacts

# Build the production binary (profile defined in Cargo.toml)
RUN --mount=type=cache,target=/build/target,id=subtensor-cargo-target \
    --mount=type=cache,target=/root/.cargo/registry,id=subtensor-cargo-registry \
    --mount=type=cache,target=/root/.cargo/git,id=subtensor-cargo-git \
    cargo build -p node-subtensor --profile production --features "metadata-hash" --locked \
    && install -m 755 /build/target/production/node-subtensor /artifacts/node-subtensor \
    && test -e /artifacts/node-subtensor

###############################################################################
# ---------- 3. Final production image (hardened) ----------------------------
###############################################################################
FROM ${BASE_IMAGE} AS subtensor

# ---- security hardening: create least-privilege user ----
RUN addgroup --system --gid 10001 subtensor && \
  adduser  --system --uid 10001 --gid 10001 --home /home/subtensor --disabled-password subtensor

# Writable data directory to be used as --base-path
RUN mkdir -p /data && chown -R subtensor:subtensor /data

# Workdir for the non-root user
WORKDIR /home/subtensor

# Copy chainspecs and binary with correct ownership
COPY --chown=subtensor:subtensor --from=prod_builder /build/*.json ./
COPY --chown=subtensor:subtensor --from=prod_builder /build/chainspecs/*.json ./chainspecs/
COPY --from=prod_builder /artifacts/node-subtensor /usr/local/bin/node-subtensor
RUN chown subtensor:subtensor /usr/local/bin/node-subtensor

EXPOSE 30333 9933 9944
USER subtensor
ENTRYPOINT ["node-subtensor"]
CMD ["--base-path","/data","--chain","mainnet"]

###############################################################################
# ---------- 4. Local build stage --------------------------------------------
###############################################################################
FROM base_builder AS local_builder

RUN mkdir -p /artifacts

# Build the workspace in release mode with the pow-faucet feature
RUN --mount=type=cache,target=/build/target,id=subtensor-cargo-target \
    --mount=type=cache,target=/root/.cargo/registry,id=subtensor-cargo-registry \
    --mount=type=cache,target=/root/.cargo/git,id=subtensor-cargo-git \
    cargo build --workspace --profile release --features "pow-faucet" \
    && install -m 755 /build/target/release/node-subtensor /artifacts/node-subtensor \
    && test -e /artifacts/node-subtensor

###############################################################################
# ---------- 5. Final local image (hardened) ----------------------------------
###############################################################################
FROM ${BASE_IMAGE} AS subtensor-local

# Least-privilege user
RUN addgroup --system --gid 10001 subtensor && \
  adduser  --system --uid 10001 --gid 10001 --home /home/subtensor --disabled-password subtensor

RUN mkdir -p /data && chown -R subtensor:subtensor /data
WORKDIR /home/subtensor

# Copy artifacts
COPY --chown=subtensor:subtensor --from=local_builder /build/*.json ./
COPY --chown=subtensor:subtensor --from=local_builder /build/chainspecs/*.json ./chainspecs/
COPY --from=local_builder /artifacts/node-subtensor /usr/local/bin/node-subtensor
RUN chown subtensor:subtensor /usr/local/bin/node-subtensor

# Generate a local chainspec for convenience (run as root before user switch)
RUN node-subtensor build-spec --disable-default-bootnode --raw --chain local > /localnet.json \
  && chown subtensor:subtensor /localnet.json

EXPOSE 30333 9933 9944
USER subtensor
ENTRYPOINT ["node-subtensor"]
CMD ["--base-path","/data","--chain","/localnet.json"]

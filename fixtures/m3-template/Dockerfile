# Two ways in, one runtime:
#
#   docker build .                          compiles the binary here (local use)
#   docker build --target from-artifact .   uses dist/hello-strictrs, already compiled
#                                           elsewhere (CI)
#
# CI takes the second path so a workflow compiles Rust exactly once instead of
# once on the runner and again inside this file. Both paths end at the same
# runtime stage, so they cannot drift.

# --- compile ----------------------------------------------------------------
FROM rust:1.97-bookworm AS builder
WORKDIR /src

# Dependencies first, against a stub source. This layer is keyed on the manifests
# alone, so editing the real code no longer recompiles the whole tree.
COPY rust-toolchain.toml Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

# tests/ is deliberately not copied: the release binary does not need it.
COPY src ./src

# Cargo keys freshness on mtime, and the stub artifact above is newer than the
# source COPY just laid down, so make the real one unambiguously newer.
RUN touch src/*.rs \
    && cargo build --release --locked --bin hello-strictrs

# --- runtime ----------------------------------------------------------------
# ca-certificates so outbound TLS can verify the server chain.
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

USER 65532:65532
ENTRYPOINT ["/usr/local/bin/hello-strictrs"]

# CI path: the binary was compiled by the workflow, in this same Debian release,
# and downloaded into dist/ — so it links against the glibc this runtime provides.
FROM runtime AS from-artifact
COPY dist/hello-strictrs /usr/local/bin/hello-strictrs

# Default path, and the last stage so a plain `docker build .` picks it.
FROM runtime AS from-source
COPY --from=builder /src/target/release/hello-strictrs /usr/local/bin/hello-strictrs

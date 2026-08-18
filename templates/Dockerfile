# Two ways in, one runtime:
#
#   docker build .                          compiles the binary here (local use)
#   docker build --target from-artifact .   uses dist/__CRATE__, already compiled
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
    && cargo build --release --locked --bin __CRATE__

# --- runtime ----------------------------------------------------------------
# Distroless: glibc (matches the bookworm builder) plus ca-certificates, with no
# shell or package manager. The :nonroot tag runs as uid 65532 by default.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
ENTRYPOINT ["/usr/local/bin/__CRATE__"]

# CI path: the binary was compiled by the workflow, in this same Debian release,
# and downloaded into dist/ — so it links against the glibc this runtime provides.
FROM runtime AS from-artifact
COPY dist/__CRATE__ /usr/local/bin/__CRATE__

# Default path, and the last stage so a plain `docker build .` picks it.
FROM runtime AS from-source
COPY --from=builder /src/target/release/__CRATE__ /usr/local/bin/__CRATE__

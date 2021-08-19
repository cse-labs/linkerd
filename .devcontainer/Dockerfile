# Note: You can use any Debian/Ubuntu base image
FROM ghcr.io/retaildevcrews/kind-rust:latest

# user args
# some base images require specific values
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

# Rust configuration for Rocket and Okapi requires rustc -nightly and rustfmt
RUN rustup default nightly && \
    rustup component add rustfmt

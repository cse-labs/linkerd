# from https://www.artificialworlds.net/blog/2020/04/22/creating-a-tiny-docker-image-of-a-rust-project/
#
# 1: Build the exe
FROM rust:1.53-slim as builder
WORKDIR /usr/src

# 1a: Prepare for static linking
RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install -y musl-tools && \
    rustup target add x86_64-unknown-linux-musl && \
    rustup component add rustfmt

# 1b: Download and compile Rust dependencies (and store as a separate Docker layer)
RUN USER=root cargo new depa
COPY rust/dill ./dill
WORKDIR /usr/src/depa
COPY rust/depa/Cargo.toml ./
RUN cargo install --target x86_64-unknown-linux-musl --path .

# 1c: Build the exe using the actual source code
COPY rust/depa/src ./src
RUN ["touch", "src/main.rs"]
RUN cargo install --target x86_64-unknown-linux-musl --path .

# 2: Copy the exe to an empty Docker image
FROM alpine:3.14
COPY --from=builder /usr/local/cargo/bin/pickle_depa .
ARG WORD_GRPC_PORT=9090
ENV WORD_GRPC_PORT=$WORD_GRPC_PORT
ARG SIGN_SVC_ADDR=http://signing-svc:9090
ENV SIGN_SVC_ADDR=$SIGN_SVC_ADDR
ARG RUST_LOG=TRACE
ENV RUST_LOG=$RUST_LOG
CMD ./pickle_depa --port $WORD_GRPC_PORT --sign-svc-addr $SIGN_SVC_ADDR

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
RUN USER=root cargo new pickle
COPY rust/b3 ./b3
COPY rust/dill ./dill
WORKDIR /usr/src/pickle
COPY rust/pickle/Cargo.toml ./
RUN cargo install --target x86_64-unknown-linux-musl --path .

# 1c: Build the exe using the actual source code
COPY rust/pickle/src ./src
RUN ["touch", "src/main.rs"]
RUN cargo install --target x86_64-unknown-linux-musl --path .

# 2: Copy the exe to an empty Docker image
FROM alpine:3.14
COPY --from=builder /usr/local/cargo/bin/pickle .
COPY rust/pickle/Rocket.toml .
ARG RUST_LOG=TRACE
ENV RUST_LOG=$RUST_LOG
EXPOSE 8000
CMD ["./pickle"]

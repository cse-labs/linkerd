# from https://www.artificialworlds.net/blog/2020/04/22/creating-a-tiny-docker-image-of-a-rust-project/
#
# 1: Build the exe
FROM rust:1.53 as builder
WORKDIR /usr/src

# 1a: Prepare for static linking
RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install -y openssl && \
    rustup component add rustfmt

# 1b: Download and compile Rust dependencies (and store as a separate Docker layer)
RUN USER=root cargo new depb
WORKDIR /usr/src/depb
COPY *.toml ./
RUN cargo install --path .

# 1c: Build the exe using the actual source code
COPY src ./src
COPY proto ./proto
COPY build.rs .
RUN ["touch", "src/main.rs"]
RUN cargo install --path .

# 2: Copy the exe to an empty Docker image
FROM rust:1.53
RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install -y openssl
COPY --from=builder /usr/local/cargo/bin/pickle_depb .
COPY keys ./keys
ENV RUST_LOG=TRACE
CMD ["./pickle_depb"]

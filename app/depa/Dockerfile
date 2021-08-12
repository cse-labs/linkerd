# from https://www.artificialworlds.net/blog/2020/04/22/creating-a-tiny-docker-image-of-a-rust-project/
#
# 1: Build the exe
FROM rust:1.53 as builder
WORKDIR /usr/src

# 1a: Prepare for static linking
RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install -y musl-tools && \
    rustup target add x86_64-unknown-linux-musl

# 1b: Download and compile Rust dependencies (and store as a separate Docker layer)
RUN USER=root cargo new depa
WORKDIR /usr/src/depa
COPY *.toml ./
RUN cargo install --target x86_64-unknown-linux-musl --path .

# 1c: Build the exe using the actual source code
COPY src ./src
RUN ["touch", "src/main.rs"]
RUN cargo install --target x86_64-unknown-linux-musl --path .

# 2: Copy the exe to an empty Docker image
FROM scratch
COPY --from=builder /usr/local/cargo/bin/depa .
CMD ["./depa"]

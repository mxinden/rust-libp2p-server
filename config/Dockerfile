FROM rust:1.51 as builder
WORKDIR /usr/src/rust-libp2p-relay-server
COPY ../ .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/libp2p-relay-server /usr/local/bin/libp2p-relay-server
CMD ["libp2p-relay-server"]
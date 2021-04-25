FROM rust:1.51 as builder
WORKDIR /usr/src/rust-libp2p-relay-server
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/libp2p-relay-server /usr/local/bin/libp2p-relay-server
CMD ["libp2p-relay-server"]
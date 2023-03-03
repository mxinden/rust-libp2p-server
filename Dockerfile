FROM rust:1.66-bullseye as builder
WORKDIR /usr/src/rust-libp2p-server

COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/libp2p-server /usr/local/bin/libp2p-server
CMD ["libp2p-server"]
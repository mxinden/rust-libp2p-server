FROM rust:1.66-bullseye as builder
WORKDIR /usr/src/rust-libp2p-server

RUN apt-get update && apt-get install -y cmake protobuf-compiler

COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/libp2p-server /usr/local/bin/libp2p-server
CMD ["libp2p-server"]
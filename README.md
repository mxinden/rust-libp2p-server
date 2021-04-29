# Rust libp2p Relay Server

A limited relay server implementing the [circuit relay
v2](https://github.com/libp2p/specs/issues/314) protocol.

See https://github.com/vyzo/libp2p-relay for the corresponding Golang
implementation.

## Usage

```
$ cargo run -- --help
libp2p relay server 0.1.0
A limited relay server implementing the circuit relay v2 protocol.

USAGE:
    libp2p-relay-server [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --identity <identity>    Identity file containing an ed25519 private key
        
$ cargo run
Local peer id: PeerId("12D3KooWDx8yJKVEN5LsCsovRb8HyHKA79cBshzShsE14ioS6Kok")
Listening for metric requests on 0.0.0.0:8080/metrics
Listening on "/ip4/127.0.0.1/tcp/4001"
```


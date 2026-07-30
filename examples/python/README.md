# Python Client Example for loci2d

This example demonstrates how a Python client communicates with the `loci2d` authoritative server using Protocol Buffers over UDP.

## Prerequisites

Install `protobuf`:
```bash
pip install protobuf
```

## Compiling the Proto Schema

Compile `proto/game_packets.proto` to generate Python bindings:
```bash
protoc -I=../../proto --python_out=. ../../proto/game_packets.proto
```
This generates `game_packets_pb2.py`.

## Running the Client

Start the server in a separate terminal:
```bash
cargo run
```

Run the Python client:
```bash
python client.py
```

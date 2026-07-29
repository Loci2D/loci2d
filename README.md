# loci2d

UDP server/client example with binary serialization using serde and bincode for 2D game networking.

## Testing

### Terminal 1 - Start the server:
```bash
cargo run
```

The server will listen on `127.0.0.1:8080` and log received packets with sequence IDs and intents.

### Terminal 2 - Start the client:
```bash
cargo run --bin client
```

### Client commands:
- `ping` - Send a ping packet
- `move <x> <y>` - Send movement intent with direction vector (e.g., `move 1.0 0.5`)
- `action <id>` - Send action intent with ability ID (e.g., `action 42`)
- `quit` - Exit the client

### Example session:
```
Enter command (move/action/ping/quit): ping
Enter command (move/action/ping/quit): move 1.0 0.5
Enter command (move/action/ping/quit): action 1
Enter command (move/action/ping/quit): quit
```

The server will deserialize binary GamePackets and respond with binary ServerResponses containing the sequence ID and acknowledgment status.

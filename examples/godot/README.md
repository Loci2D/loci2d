# Godot (GDScript) Client Example for loci2d

This example demonstrates how a **Godot engine** project connects to the `loci2d` authoritative Rust server using `PacketPeerUDP` and GDScript Protobuf serialization.

## Prerequisites

- **Godot 4.x**: Download from [godotengine.org](https://godotengine.org/).
- **godot-protobuf / protobuf.gd**: GDScript Protobuf plugin (e.g. `godot-protobuf` or `protobuf.gd`).

## Implementation Overview (`NetworkClient.gd`)

In Godot, networking with `loci2d` uses standard UDP sockets combined with binary Protobuf data:

```gdscript
extends Node

var udp := PacketPeerUDP.new()
var server_ip := "127.0.0.1"
var server_port := 8080
var sequence_id : int = 0

func _ready() -> void:
	udp.connect_to_host(server_ip, server_port)
	print("[Godot] Connected to loci2d server at ", server_ip, ":", server_port)

func send_move_intent(direction: Vector2) -> void:
	sequence_id += 1
	
	# Construct Protobuf GamePacket
	var packet = GamePacket.new()
	packet.set_sequence_id(sequence_id)
	packet.set_timestamp(Time.get_unix_time_from_system())
	
	var move = MoveIntent.new()
	var vec = Vector2Proto.new()
	vec.set_x(direction.x)
	vec.set_y(direction.y)
	move.set_direction(vec)
	
	var intent = ClientIntent.new()
	intent.set_move(move)
	packet.set_intent(intent)
	
	# Send serialized byte array over UDP
	var bytes: PackedByteArray = packet.to_bytes()
	udp.put_packet(bytes)

func _process(_delta: float) -> void:
	while udp.get_available_packet_count() > 0:
		var packet_bytes: PackedByteArray = udp.get_packet()
		var response = ServerResponse.new()
		response.from_bytes(packet_bytes)
		print("[Godot] Received ACK: ", response.get_status())
```

## Running the Example

1. Start the Rust server:
```bash
cargo run
```

2. Open the Godot project in `examples/godot/` and run the main scene.

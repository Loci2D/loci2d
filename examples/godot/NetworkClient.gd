# NetworkClient.gd - Godot 4 GDScript example for loci2d UDP server
extends Node

@export var server_host: String = "127.0.0.1"
@export var server_port: int = 8080

var _udp := PacketPeerUDP.new()
var _sequence_id: int = 0

func _ready() -> void:
	var err = _udp.connect_to_host(server_host, server_port)
	if err == OK:
		print("[loci2d NetworkClient] Connected UDP to ", server_host, ":", server_port)
	else:
		printerr("[loci2d NetworkClient] Failed to connect UDP to host, error code: ", err)

func _process(_delta: float) -> void:
	# Poll for incoming UDP responses from loci2d server
	while _udp.get_available_packet_count() > 0:
		var raw_bytes: PackedByteArray = _udp.get_packet()
		_handle_server_response(raw_bytes)

func send_join(player_name: String = "GodotPlayer") -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Join name=", player_name, " (seq=", _sequence_id, ")")

func send_disconnect(reason: String = "normal quit") -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Disconnect reason=", reason, " (seq=", _sequence_id, ")")

func send_ping() -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Ping (seq=", _sequence_id, ")")
	# In actual GDScript using protobuf plugin:
	# var packet = GamePacket.new()
	# packet.set_sequence_id(_sequence_id)
	# ...
	# _udp.put_packet(packet.to_bytes())

func send_move(direction: Vector2) -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Move direction=", direction, " (seq=", _sequence_id, ")")

func send_action(ability_id: int) -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Action ability_id=", ability_id, " (seq=", _sequence_id, ")")

func _handle_server_response(bytes: PackedByteArray) -> void:
	print("[loci2d NetworkClient] Received ", bytes.size(), " bytes response from server")
	# In actual GDScript using protobuf plugin:
	# var response = ServerResponse.new()
	# response.from_bytes(bytes)
	# print("ACK seq: ", response.get_sequence_id(), " status: ", response.get_status())

# NetworkClient.gd - Godot 4 GDScript example for loci2d UDP server
extends Node

signal world_state_updated(tick: int, entities: Array)
signal server_response_received(sequence_id: int, status: String)

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
	# Poll for incoming UDP responses/snapshots from loci2d server
	while _udp.get_available_packet_count() > 0:
		var raw_bytes: PackedByteArray = _udp.get_packet()
		_handle_server_packet(raw_bytes)

func send_join(player_name: String = "GodotPlayer") -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Join name=", player_name, " (seq=", _sequence_id, ")")

func send_disconnect(reason: String = "normal quit") -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Disconnect reason=", reason, " (seq=", _sequence_id, ")")

func send_ping() -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Ping (seq=", _sequence_id, ")")

func send_move(direction: Vector2) -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Move direction=", direction, " (seq=", _sequence_id, ")")

func send_action(ability_id: int) -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Action ability_id=", ability_id, " (seq=", _sequence_id, ")")

func _handle_server_packet(bytes: PackedByteArray) -> void:
	print("[loci2d NetworkClient] Received ", bytes.size(), " bytes packet from server")
	# In actual GDScript using protobuf plugin:
	# var packet = ServerPacket.new()
	# packet.from_bytes(bytes)
	# if packet.has_world_state():
	#     var ws = packet.get_world_state()
	#     world_state_updated.emit(ws.get_tick(), ws.get_entities())
	# elif packet.has_response():
	#     var resp = packet.get_response()
	#     server_response_received.emit(resp.get_sequence_id(), resp.get_status())

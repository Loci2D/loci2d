# NetworkClient.gd - Godot 4 GDScript example for loci2d UDP server & Replay Spectator
extends Node

signal world_state_updated(tick: int, entities: Array)
signal server_response_received(sequence_id: int, status: String)
signal stream_disconnected()

@export var server_host: String = "127.0.0.1"
@export var server_port: int = 8080
@export var is_spectator: bool = false

var _udp := PacketPeerUDP.new()
var _sequence_id: int = 0
var _last_packet_time: float = 0.0
var _last_heartbeat_time: float = 0.0

func _ready() -> void:
	var err = _udp.connect_to_host(server_host, server_port)
	if err == OK:
		print("[loci2d NetworkClient] Connected UDP to ", server_host, ":", server_port)
		if is_spectator:
			print("[loci2d NetworkClient] Mode: SPECTATOR (Replay Viewer)")
			send_ping()
		else:
			send_join("GodotPlayer")
	else:
		printerr("[loci2d NetworkClient] Failed to connect UDP to host, error code: ", err)

func _process(delta: float) -> void:
	var now = Time.get_ticks_msec() / 1000.0

	# Maintain periodic heartbeat in spectator mode
	if is_spectator and (now - _last_heartbeat_time >= 1.0):
		_last_heartbeat_time = now
		send_ping()

	# Poll for incoming UDP responses/snapshots from loci2d server
	var has_packets = false
	while _udp.get_available_packet_count() > 0:
		var raw_bytes: PackedByteArray = _udp.get_packet()
		_last_packet_time = now
		has_packets = true
		_handle_server_packet(raw_bytes)

	# Connection timeout detection (> 2.0s without packets)
	if _last_packet_time > 0 and (now - _last_packet_time > 2.0):
		stream_disconnected.emit()
		_last_packet_time = 0

func start_spectating() -> void:
	is_spectator = true
	send_ping()
	print("[loci2d NetworkClient] Switched to Spectator / Replay Mode")

func send_join(player_name: String = "GodotPlayer") -> void:
	is_spectator = false
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Join name=", player_name, " (seq=", _sequence_id, ")")

func send_disconnect(reason: String = "normal quit") -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Disconnect reason=", reason, " (seq=", _sequence_id, ")")

func send_ping() -> void:
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Ping (seq=", _sequence_id, ")")

func send_move(direction: Vector2) -> void:
	if is_spectator:
		return
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Move direction=", direction, " (seq=", _sequence_id, ")")
	# In actual GDScript protobuf usage for Phase 5 fixed-point math:
	# var move_intent = packet.get_intent().get_move()
	# move_intent.get_direction().set_x_bits(int(direction.x * 65536))
	# move_intent.get_direction().set_y_bits(int(direction.y * 65536))

func send_action(ability_id: int) -> void:
	if is_spectator:
		return
	_sequence_id += 1
	print("[loci2d NetworkClient] Sending Action ability_id=", ability_id, " (seq=", _sequence_id, ")")

func _handle_server_packet(bytes: PackedByteArray) -> void:
	print("[loci2d NetworkClient] Received ", bytes.size(), " bytes packet from server")
	# In actual GDScript using protobuf plugin:
	# var packet = ServerPacket.new()
	# packet.from_bytes(bytes)
	# if packet.has_world_state():
	#     var ws = packet.get_world_state()
	#     # Convert from I16F16 fixed point bits to Godot float coords:
	#     # for entity in ws.get_entities():
	#     #     var px = entity.get_position().get_x_bits() / 65536.0
	#     #     var py = entity.get_position().get_y_bits() / 65536.0
	#     world_state_updated.emit(ws.get_tick(), ws.get_entities())
	# elif packet.has_response():
	#     var resp = packet.get_response()
	#     server_response_received.emit(resp.get_sequence_id(), resp.get_status())

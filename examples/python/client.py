import socket
import sys
import threading
import time

try:
    # pyrefly: ignore [missing-import]
    import game_packets_pb2
except ImportError:
    print("Error: game_packets_pb2.py not found.")
    print("Please generate it first by running:")
    print("  protoc -I=../../proto --python_out=. ../../proto/game_packets.proto")
    sys.exit(1)

SERVER_ADDR = ("127.0.0.1", 8080)

latest_world_state = None
latest_lock = threading.Lock()
stream_enabled = False
is_spectator = False
last_packet_time = 0.0

def print_world_state(ws):
    print(f"\n--- [World State Snapshot | Tick {ws.tick} | Timestamp: {ws.timestamp}] ---")
    if not ws.entities:
        print("  (No active entities in instance)")
    else:
        print(f"  Active Entities ({len(ws.entities)}):")
        for e in ws.entities:
            type_name = game_packets_pb2.EntityType.Name(e.entity_type)
            print(f"    - Entity {e.id} (\"{e.name}\", {type_name}) @ ({e.position.x:.1f}, {e.position.y:.1f}), vel=({e.velocity.x:.1f}, {e.velocity.y:.1f})")
    print("---------------------------------------------------------")

def listen_server(sock, stop_event):
    global latest_world_state, stream_enabled, last_packet_time
    sock.settimeout(0.5)
    last_stream_print = 0.0

    while not stop_event.is_set():
        try:
            data, _ = sock.recvfrom(2048)
            last_packet_time = time.time()
            server_packet = game_packets_pb2.ServerPacket()
            server_packet.ParseFromString(data)

            if server_packet.HasField("world_state"):
                ws = server_packet.world_state
                with latest_lock:
                    latest_world_state = ws
                now = time.time()
                if stream_enabled and (now - last_stream_print >= 1.0):
                    print_world_state(ws)
                    last_stream_print = now
            elif server_packet.HasField("response"):
                resp = server_packet.response
                print(f"\n[Server Response] ACK seq={resp.sequence_id} status={resp.status}")
        except socket.timeout:
            now = time.time()
            if last_packet_time > 0 and (now - last_packet_time > 2.0):
                with latest_lock:
                    if latest_world_state is not None and len(latest_world_state.entities) > 0:
                        latest_world_state = None
                        print("\n[Stream Status] Server disconnected or replay finished (stream silent).")
            continue
        except Exception as e:
            if not stop_event.is_set():
                print(f"\n[Error receiving packet] {e}")

def spectator_heartbeat(sock, stop_event):
    """Periodically sends PingIntent to keep spectator registration active."""
    seq = 100000
    while not stop_event.is_set():
        try:
            packet = game_packets_pb2.GamePacket()
            packet.sequence_id = seq
            packet.timestamp = int(time.time() * 1000)
            seq += 1
            packet.intent.ping.CopyFrom(game_packets_pb2.PingIntent())
            data = packet.SerializeToString()
            sock.sendto(data, SERVER_ADDR)
        except Exception:
            pass
        time.sleep(1.0)

def main():
    global stream_enabled, is_spectator
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    
    # Check if launched with --spectate or --replay
    if len(sys.argv) > 1 and ("--spectate" in sys.argv or "--replay" in sys.argv or "-s" in sys.argv):
        is_spectator = True
        stream_enabled = True

    stop_event = threading.Event()
    listener_thread = threading.Thread(target=listen_server, args=(sock, stop_event), daemon=True)
    listener_thread.start()

    if is_spectator:
        heartbeat_thread = threading.Thread(target=spectator_heartbeat, args=(sock, stop_event), daemon=True)
        heartbeat_thread.start()
        print("[Python Client] Started in SPECTATOR / REPLAY mode -> Watching match stream from 127.0.0.1:8080")
    else:
        print("[Python Client] Ready to connect to loci2d server at 127.0.0.1:8080")

    print("Commands: join <name> | spectate | status | move <x> <y> | stream <on|off> | leave [reason] | action <id> | ping | quit")

    sequence_id = 0
    while True:
        try:
            cmd = input("Enter command: ").strip()
        except (EOFError, KeyboardInterrupt):
            break

        if not cmd:
            continue

        if cmd == "spectate":
            is_spectator = True
            stream_enabled = True
            heartbeat_thread = threading.Thread(target=spectator_heartbeat, args=(sock, stop_event), daemon=True)
            heartbeat_thread.start()
            print("[Spectator] Switched to Spectator mode -> Periodic heartbeats and stream logging enabled.")
            continue

        if cmd == "status" or cmd == "state" or cmd == "entities":
            with latest_lock:
                ws = latest_world_state
            if ws:
                print_world_state(ws)
            else:
                print("[Status] No active world state snapshot (server idle or disconnected).")
            continue

        if cmd == "stream on":
            stream_enabled = True
            print("[Stream] Live snapshot logging ENABLED (throttled to 1s).")
            continue

        if cmd == "stream off":
            stream_enabled = False
            print("[Stream] Live snapshot logging DISABLED. Use 'status' to inspect world state.")
            continue

        packet = game_packets_pb2.GamePacket()
        packet.sequence_id = sequence_id
        packet.timestamp = int(time.time() * 1000)
        sequence_id += 1

        if cmd == "quit":
            if not is_spectator:
                packet.intent.disconnect.reason = "normal quit"
                data = packet.SerializeToString()
                sock.sendto(data, SERVER_ADDR)
            print(f"[Quit] Shutting down client...")
            stop_event.set()
            break
        elif cmd.startswith("join"):
            parts = cmd.split(maxsplit=1)
            player_name = parts[1] if len(parts) > 1 else "PythonPlayer"
            packet.intent.join.player_name = player_name
        elif cmd.startswith("leave"):
            parts = cmd.split(maxsplit=1)
            reason = parts[1] if len(parts) > 1 else "leaving session"
            packet.intent.disconnect.reason = reason
        elif cmd == "ping":
            packet.intent.ping.CopyFrom(game_packets_pb2.PingIntent())
        elif cmd.startswith("move"):
            parts = cmd.split()
            x = float(parts[1]) if len(parts) > 1 else 1.0
            y = float(parts[2]) if len(parts) > 2 else 0.0
            packet.intent.move.direction.x = x
            packet.intent.move.direction.y = y
        elif cmd.startswith("action"):
            parts = cmd.split()
            ability_id = int(parts[1]) if len(parts) > 1 else 1
            packet.intent.action.ability_id = ability_id
        else:
            print("Unknown command. Available: join <name>, spectate, status, move <x> <y>, stream <on|off>, leave [reason], action <id>, ping, quit")
            continue

        # Serialize packet to binary bytes
        data = packet.SerializeToString()
        sock.sendto(data, SERVER_ADDR)
        print(f"[Sent] {len(data)} bytes | sequence_id={packet.sequence_id}")

if __name__ == "__main__":
    main()

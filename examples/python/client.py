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

def listen_server(sock, stop_event):
    sock.settimeout(0.5)
    while not stop_event.is_set():
        try:
            data, _ = sock.recvfrom(2048)
            server_packet = game_packets_pb2.ServerPacket()
            server_packet.ParseFromString(data)

            if server_packet.HasField("world_state"):
                ws = server_packet.world_state
                entity_strs = []
                for e in ws.entities:
                    type_name = game_packets_pb2.EntityType.Name(e.entity_type)
                    entity_strs.append(f"{e.name} (id={e.id}, {type_name}) @ ({e.position.x:.1f}, {e.position.y:.1f})")
                print(f"\n[Snapshot Tick {ws.tick}] {len(ws.entities)} entity/entities: " + ", ".join(entity_strs))
            elif server_packet.HasField("response"):
                resp = server_packet.response
                print(f"\n[Server Response] ACK seq={resp.sequence_id} status={resp.status}")
        except socket.timeout:
            continue
        except Exception as e:
            if not stop_event.is_set():
                print(f"\n[Error receiving packet] {e}")

def main():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    
    stop_event = threading.Event()
    listener_thread = threading.Thread(target=listen_server, args=(sock, stop_event), daemon=True)
    listener_thread.start()

    sequence_id = 0
    print("[Python Client] Ready to connect to loci2d server at 127.0.0.1:8080")
    print("Commands: join <name> | leave [reason] | move <x> <y> | action <id> | ping | quit")

    while True:
        try:
            cmd = input("Enter command: ").strip()
        except (EOFError, KeyboardInterrupt):
            break

        if not cmd:
            continue

        packet = game_packets_pb2.GamePacket()
        packet.sequence_id = sequence_id
        packet.timestamp = int(time.time() * 1000)
        sequence_id += 1

        if cmd == "quit":
            packet.intent.disconnect.reason = "normal quit"
            data = packet.SerializeToString()
            sock.sendto(data, SERVER_ADDR)
            print(f"[Sent] Disconnect packet ({len(data)} bytes) | Shutting down...")
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
            print("Unknown command. Available: join <name>, leave [reason], move <x> <y>, action <id>, ping, quit")
            continue

        # Serialize packet to binary bytes
        data = packet.SerializeToString()
        sock.sendto(data, SERVER_ADDR)
        print(f"[Sent] {len(data)} bytes | sequence_id={packet.sequence_id}")

if __name__ == "__main__":
    main()

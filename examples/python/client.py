import socket
import sys
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

def main():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(5.0)
    
    sequence_id = 0
    print("[Python Client] Connected to loci2d server at 127.0.0.1:8080")

    while True:
        try:
            cmd = input("Enter command (ping/move <x> <y>/action <id>/quit): ").strip()
        except (EOFError, KeyboardInterrupt):
            break

        if cmd == "quit":
            break
        if not cmd:
            continue

        packet = game_packets_pb2.GamePacket()
        packet.sequence_id = sequence_id
        packet.timestamp = int(time.time())
        sequence_id += 1

        if cmd == "ping":
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
            packet.intent.ping.CopyFrom(game_packets_pb2.PingIntent())

        # Serialize packet to binary bytes
        data = packet.SerializeToString()
        sock.sendto(data, SERVER_ADDR)
        print(f"[Sent] {len(data)} bytes | sequence_id={packet.sequence_id}")

        try:
            response_data, _ = sock.recvfrom(1024)
            response = game_packets_pb2.ServerResponse()
            response.ParseFromString(response_data)
            print(f"[Received] {len(response_data)} bytes | ACK sequence_id={response.sequence_id} | status={response.status}")
        except socket.timeout:
            print("[Timeout] No response received from server")

if __name__ == "__main__":
    main()

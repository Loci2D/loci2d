// [2026-08-08] Allowed dead_code: includes Protobuf types (e.g. ServerResponse) generated via prost.
#![allow(dead_code)]

// Importa os tipos gerados pelo prost a partir do proto/game_packets.proto
include!(concat!(env!("OUT_DIR"), "/loci2d.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn test_protobuf_packet_roundtrip() {
        let packet = GamePacket {
            sequence_id: 42,
            timestamp: 1000,
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Move(MoveIntent {
                    direction: Some(Vector2 { x: 1.0, y: -2.5 }),
                })),
            }),
        };

        let mut buf = Vec::new();
        packet.encode(&mut buf).expect("Failed to encode packet");

        let decoded = GamePacket::decode(&buf[..]).expect("Failed to decode packet");
        assert_eq!(decoded.sequence_id, 42);
        assert_eq!(decoded.timestamp, 1000);

        match decoded.intent {
            Some(ClientIntent { intent: Some(client_intent::Intent::Move(m)) }) => {
                let dir = m.direction.expect("Missing direction");
                assert_eq!(dir.x, 1.0);
                assert_eq!(dir.y, -2.5);
            }
            _ => panic!("Expected Move intent"),
        }
    }
}

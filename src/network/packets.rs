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

    #[test]
    fn test_protobuf_join_and_disconnect_roundtrip() {
        // Join intent
        let join_packet = GamePacket {
            sequence_id: 1,
            timestamp: 2000,
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Join(JoinIntent {
                    player_name: "Alice".to_string(),
                })),
            }),
        };
        let mut buf = Vec::new();
        join_packet.encode(&mut buf).unwrap();
        let decoded_join = GamePacket::decode(&buf[..]).unwrap();
        match decoded_join.intent {
            Some(ClientIntent { intent: Some(client_intent::Intent::Join(j)) }) => {
                assert_eq!(j.player_name, "Alice");
            }
            _ => panic!("Expected Join intent"),
        }

        // Disconnect intent
        let dc_packet = GamePacket {
            sequence_id: 2,
            timestamp: 2001,
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Disconnect(DisconnectIntent {
                    reason: "Leaving match".to_string(),
                })),
            }),
        };
        let mut dc_buf = Vec::new();
        dc_packet.encode(&mut dc_buf).unwrap();
        let decoded_dc = GamePacket::decode(&dc_buf[..]).unwrap();
        match decoded_dc.intent {
            Some(ClientIntent { intent: Some(client_intent::Intent::Disconnect(d)) }) => {
                assert_eq!(d.reason, "Leaving match");
            }
            _ => panic!("Expected Disconnect intent"),
        }
    }
}


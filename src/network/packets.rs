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

    #[test]
    fn test_protobuf_server_packet_world_state_roundtrip() {
        let server_packet = ServerPacket {
            sequence_id: 100,
            payload: Some(server_packet::Payload::WorldState(WorldState {
                tick: 100,
                timestamp: 1723140000000,
                entities: vec![
                    EntityState {
                        id: 1,
                        name: "Arthur".to_string(),
                        position: Some(Vector2 { x: 10.5, y: -20.0 }),
                        velocity: Some(Vector2 { x: 1.0, y: 0.0 }),
                        entity_type: EntityType::Player as i32,
                    },
                    EntityState {
                        id: 2,
                        name: "Goblin".to_string(),
                        position: Some(Vector2 { x: 50.0, y: 30.0 }),
                        velocity: Some(Vector2 { x: 0.0, y: 0.0 }),
                        entity_type: EntityType::Npc as i32,
                    },
                ],
            })),
        };

        let mut buf = Vec::new();
        server_packet.encode(&mut buf).expect("Failed to encode server packet");

        let decoded = ServerPacket::decode(&buf[..]).expect("Failed to decode server packet");
        assert_eq!(decoded.sequence_id, 100);

        match decoded.payload {
            Some(server_packet::Payload::WorldState(ws)) => {
                assert_eq!(ws.tick, 100);
                assert_eq!(ws.timestamp, 1723140000000);
                assert_eq!(ws.entities.len(), 2);

                let e1 = &ws.entities[0];
                assert_eq!(e1.id, 1);
                assert_eq!(e1.name, "Arthur");
                assert_eq!(e1.entity_type, EntityType::Player as i32);
                let pos1 = e1.position.as_ref().unwrap();
                assert_eq!(pos1.x, 10.5);
                assert_eq!(pos1.y, -20.0);

                let e2 = &ws.entities[1];
                assert_eq!(e2.id, 2);
                assert_eq!(e2.name, "Goblin");
                assert_eq!(e2.entity_type, EntityType::Npc as i32);
            }
            _ => panic!("Expected WorldState payload"),
        }
    }
}


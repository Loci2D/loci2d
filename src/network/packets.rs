use serde::{Serialize, Deserialize};

// Vetor 2D simples para o mapa 2D
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

// O que o cliente *pode querer fazer* (Intenções)
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientIntent {
    // Intenção de movimento informando uma direção ou delta
    Move { direction: Vector2 },
    
    // Outras intenções comuns em jogos
    Action { ability_id: u32 },
    
    // Heartbeat/Ping para manter a conexão viva
    Ping,
}

// O pacote completo que trafega na rede (Envelope)
#[derive(Serialize, Deserialize, Debug)]
pub struct GamePacket {
    pub sequence_id: u64, // Útil para ordenar pacotes e evitar replay attacks básico
    pub intent: ClientIntent,
}

// Resposta do servidor
#[derive(Serialize, Deserialize, Debug)]
pub struct ServerResponse {
    pub sequence_id: u64,
    pub status: String,
}

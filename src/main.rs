// Ponto de entrada: inicializa o runtime, socket e arranca o loop principal

mod network;
mod world;
mod game_loop;
mod scripting;

use network::run_server;

fn main() {
    run_server();
}

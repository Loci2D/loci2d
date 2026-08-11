fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc_path = protoc_bin_vendored::protoc_bin_path()?;
    // Safety: single-threaded build script execution before worker threads are spawned
    unsafe {
        std::env::set_var("PROTOC", protoc_path);
    }
    prost_build::compile_protos(
        &["proto/game_packets.proto", "proto/replay.proto"],
        &["proto/"],
    )?;
    Ok(())
}

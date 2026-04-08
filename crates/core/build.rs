fn main() {
    let fds = protox::compile(["proto/gtfs-realtime.proto"], ["proto/"])
        .expect("protobuf compilation failed");
    prost_build::Config::new()
        .compile_fds(fds)
        .expect("prost code generation failed");
}

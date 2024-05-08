use std::io::Result;

use poem_grpc_build::compile_protos;
fn main() -> Result<()> {
    std::env::set_var("OUT_DIR", "tests/grpc/rust");
    compile_protos(&["./tests/grpc/proto/helloworld.proto"], &["./tests/grpc/proto/"]).expect("fail to build");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../dill/proto/dill/dill.proto")?;
    Ok(())
}

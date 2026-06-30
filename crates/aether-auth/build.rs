//! Build script for aether-auth.
#![allow(missing_docs)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure().compile_protos(
        &["../../proto/aether.proto", "../../proto/csi.proto"],
        &["../../proto"],
    )?;
    Ok(())
}

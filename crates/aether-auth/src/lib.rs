// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

/// Module for handling Mutual TLS (mTLS) configuration and handshake checks.
pub mod mtls;

/// Module for generating and validating ephemeral attestation tokens.
pub mod token;

/// Auto-generated tonic/prost gRPC modules.
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
#[allow(clippy::all)]
pub mod proto {
    tonic::include_proto!("aether");
}

/// Auto-generated tonic/prost CSI gRPC modules.
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
#[allow(clippy::all)]
pub mod csi {
    tonic::include_proto!("csi.v1");
}

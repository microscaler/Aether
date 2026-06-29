use tonic::transport::{Certificate, ClientTlsConfig, Identity, ServerTlsConfig};

/// Creates a server TLS configuration enforcing mTLS.
/// Rejects any connection without a client certificate signed by the provided CA.
pub fn create_server_tls_config(
    ca_cert_pem: &[u8],
    server_cert_pem: &[u8],
    server_key_pem: &[u8],
) -> ServerTlsConfig {
    let client_ca = Certificate::from_pem(ca_cert_pem);
    let identity = Identity::from_pem(server_cert_pem, server_key_pem);
    ServerTlsConfig::new()
        .identity(identity)
        .client_ca_root(client_ca)
}

/// Creates a client TLS configuration for mTLS connection.
/// Configures client identity and CA certificate for server verification.
pub fn create_client_tls_config(
    ca_cert_pem: &[u8],
    client_cert_pem: &[u8],
    client_key_pem: &[u8],
    domain_name: &str,
) -> ClientTlsConfig {
    let server_ca = Certificate::from_pem(ca_cert_pem);
    let identity = Identity::from_pem(client_cert_pem, client_key_pem);
    ClientTlsConfig::new()
        .ca_certificate(server_ca)
        .identity(identity)
        .domain_name(domain_name)
}

/// Dynamic test helper module for generating self-signed certificates.
pub mod test_pki {
    use rcgen::{CertificateParams, DnType, IsCa, Issuer, KeyPair, SanType};

    /// Struct containing generated PEM certificates and keys.
    pub struct GeneratedCreds {
        /// CA Certificate PEM
        pub ca_cert: Vec<u8>,
        /// Server Certificate PEM
        pub server_cert: Vec<u8>,
        /// Server Key PEM
        pub server_key: Vec<u8>,
        /// Client Certificate PEM
        pub client_cert: Vec<u8>,
        /// Client Key PEM
        pub client_key: Vec<u8>,
    }

    /// Generates certificates and keys for testing mTLS connections.
    /// Returns credentials as PEM bytes.
    pub fn generate_test_creds() -> Result<GeneratedCreds, Box<dyn std::error::Error>> {
        // Generate CA key and params
        let ca_keypair = KeyPair::generate()?;
        let mut ca_params = CertificateParams::default();
        ca_params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        ca_params
            .distinguished_name
            .push(DnType::CommonName, "Aether Test CA");
        let ca_cert = ca_params.self_signed(&ca_keypair)?;
        let ca_cert_pem = ca_cert.pem().into_bytes();

        // Generate server key and params
        let server_keypair = KeyPair::generate()?;
        let mut server_params = CertificateParams::default();
        server_params
            .distinguished_name
            .push(DnType::CommonName, "localhost");
        server_params
            .subject_alt_names
            .push(SanType::DnsName("localhost".try_into()?));
        server_params
            .subject_alt_names
            .push(SanType::IpAddress("127.0.0.1".parse()?));
        let ca_issuer = Issuer::new(ca_params.clone(), &ca_keypair);
        let server_cert = server_params.signed_by(&server_keypair, &ca_issuer)?;
        let server_cert_pem = server_cert.pem().into_bytes();
        let server_key_pem = server_keypair.serialize_pem().into_bytes();

        // Generate client key and params
        let client_keypair = KeyPair::generate()?;
        let mut client_params = CertificateParams::default();
        client_params
            .distinguished_name
            .push(DnType::CommonName, "aetherd-client");
        let client_cert = client_params.signed_by(&client_keypair, &ca_issuer)?;
        let client_cert_pem = client_cert.pem().into_bytes();
        let client_key_pem = client_keypair.serialize_pem().into_bytes();

        Ok(GeneratedCreds {
            ca_cert: ca_cert_pem,
            server_cert: server_cert_pem,
            server_key: server_key_pem,
            client_cert: client_cert_pem,
            client_key: client_key_pem,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_mtls_configs_generation() {
        let creds = test_pki::generate_test_creds().unwrap();

        // Create server TLS config
        let _server_config =
            create_server_tls_config(&creds.ca_cert, &creds.server_cert, &creds.server_key);
        // We can't easily inspect the internal fields, but compiling/building confirms it's valid.

        // Create client TLS config
        let _client_config = create_client_tls_config(
            &creds.ca_cert,
            &creds.client_cert,
            &creds.client_key,
            "localhost",
        );
    }
}

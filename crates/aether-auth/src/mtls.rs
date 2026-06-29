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
    use rcgen::{Certificate, CertificateParams, DnType, IsCa, KeyPair, SanType};

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
        // Generate CA
        let mut ca_params = CertificateParams::default();
        ca_params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        ca_params
            .distinguished_name
            .push(DnType::CommonName, "Aether Test CA");
        let ca_keypair = KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)?;
        ca_params.key_pair = Some(ca_keypair);
        let ca_cert = Certificate::from_params(ca_params)?;
        let ca_cert_pem = ca_cert.serialize_pem()?.into_bytes();

        // Generate Server Cert signed by CA
        let mut server_params = CertificateParams::default();
        server_params
            .distinguished_name
            .push(DnType::CommonName, "localhost");
        server_params
            .subject_alt_names
            .push(SanType::DnsName("localhost".to_string()));
        server_params
            .subject_alt_names
            .push(SanType::IpAddress("127.0.0.1".parse()?));
        let server_keypair = KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)?;
        server_params.key_pair = Some(server_keypair);
        let server_cert = Certificate::from_params(server_params)?;
        let server_cert_pem = server_cert
            .serialize_pem_with_signer(&ca_cert)?
            .into_bytes();
        let server_key_pem = server_cert.serialize_private_key_pem().into_bytes();

        // Generate Client Cert signed by CA
        let mut client_params = CertificateParams::default();
        client_params
            .distinguished_name
            .push(DnType::CommonName, "aetherd-client");
        let client_keypair = KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)?;
        client_params.key_pair = Some(client_keypair);
        let client_cert = Certificate::from_params(client_params)?;
        let client_cert_pem = client_cert
            .serialize_pem_with_signer(&ca_cert)?
            .into_bytes();
        let client_key_pem = client_cert.serialize_private_key_pem().into_bytes();

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

// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::io::BufReader;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::rustls::{ClientConfig, RootCertStore, ServerConfig};

fn parse_certs(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>, String> {
    let mut reader = BufReader::new(pem);
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to parse certificates: {}", e))?;
    Ok(certs)
}

fn parse_key(pem: &[u8]) -> Result<PrivateKeyDer<'static>, String> {
    let mut reader = BufReader::new(pem);
    let key = rustls_pemfile::private_key(&mut reader)
        .map_err(|e| format!("Failed to parse private key: {}", e))?
        .ok_or_else(|| "No private key found in PEM".to_string())?;
    Ok(key)
}

/// Creates a ClientConfig wrapper supporting mTLS over UDS stream.
pub fn create_vsock_client_config(
    ca_pem: &[u8],
    client_cert_pem: &[u8],
    client_key_pem: &[u8],
) -> Result<ClientConfig, String> {
    let root_certs = parse_certs(ca_pem)?;
    let mut root_store = RootCertStore::empty();
    for cert in root_certs {
        root_store
            .add(cert)
            .map_err(|e| format!("Failed to add CA certificate: {}", e))?;
    }

    let certs = parse_certs(client_cert_pem)?;
    let key = parse_key(client_key_pem)?;

    let provider = tokio_rustls::rustls::crypto::ring::default_provider();
    let config = ClientConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("Failed to setup protocol versions: {:?}", e))?
        .with_root_certificates(root_store)
        .with_client_auth_cert(certs, key)
        .map_err(|e| format!("Failed to configure client authentication: {}", e))?;

    Ok(config)
}

/// Creates a ServerConfig wrapper enforcing mTLS over UDS stream (primarily for testing/mocking).
pub fn create_vsock_server_config(
    ca_pem: &[u8],
    server_cert_pem: &[u8],
    server_key_pem: &[u8],
) -> Result<ServerConfig, String> {
    let client_ca_certs = parse_certs(ca_pem)?;
    let mut client_roots = RootCertStore::empty();
    for cert in client_ca_certs {
        client_roots
            .add(cert)
            .map_err(|e| format!("Failed to add client root CA: {}", e))?;
    }

    let verifier = tokio_rustls::rustls::server::WebPkiClientVerifier::builder(client_roots.into())
        .build()
        .map_err(|e| format!("Failed to build client cert verifier: {}", e))?;

    let certs = parse_certs(server_cert_pem)?;
    let key = parse_key(server_key_pem)?;

    let provider = tokio_rustls::rustls::crypto::ring::default_provider();
    let config = ServerConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("Failed to setup protocol versions: {:?}", e))?
        .with_client_cert_verifier(verifier)
        .with_single_cert(certs, key)
        .map_err(|e| format!("Failed to configure single certificate: {}", e))?;

    Ok(config)
}

/// A connector structure that manages connecting to the guest VM over the UDS socket.
pub struct VsockConnector {
    /// Hostpath to the multiplexer Unix domain socket.
    pub socket_path: String,
}

impl VsockConnector {
    /// Creates a new instance of `VsockConnector`.
    pub fn new(socket_path: String) -> Self {
        Self { socket_path }
    }

    /// Establishes a raw connection to a guest port over the host Unix domain socket.
    pub async fn connect_to_guest(&self, port: u32) -> Result<tokio::net::UnixStream, String> {
        let mut stream = tokio::net::UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| format!("Failed to connect to UDS at '{}': {}", self.socket_path, e))?;

        let request = format!("CONNECT {}\n", port);
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| format!("Failed to dispatch CONNECT handshake: {}", e))?;

        let mut response_bytes = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            stream
                .read_exact(&mut byte)
                .await
                .map_err(|e| format!("Failed to read UDS handshake byte: {}", e))?;
            response_bytes.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        let response = String::from_utf8(response_bytes)
            .map_err(|e| format!("Invalid UTF-8 handshake response: {}", e))?;

        if response.starts_with("OK") {
            Ok(stream)
        } else {
            Err(format!(
                "Handshake rejected by Firecracker multiplexer: {}",
                response.trim()
            ))
        }
    }

    /// Establishes a TLS-encrypted connection to a guest port over the host Unix domain socket.
    pub async fn connect_to_guest_secure(
        &self,
        port: u32,
        tls_config: Arc<ClientConfig>,
        domain: &str,
    ) -> Result<tokio_rustls::client::TlsStream<tokio::net::UnixStream>, String> {
        let raw_stream = self.connect_to_guest(port).await?;
        let connector = tokio_rustls::TlsConnector::from(tls_config);
        let server_name = rustls_pki_types::ServerName::try_from(domain)
            .map_err(|e| format!("Invalid server domain name '{}': {}", domain, e))?
            .to_owned();

        let secure_stream = connector
            .connect(server_name, raw_stream)
            .await
            .map_err(|e| format!("TLS handshake failed over VSOCK stream: {}", e))?;

        Ok(secure_stream)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use aether_auth::mtls::test_pki::generate_test_creds;
    use tempfile::tempdir;
    use tokio::io::AsyncBufReadExt;

    #[tokio::test]
    async fn test_vsock_handshake_success() {
        let dir = tempdir().unwrap();
        let sock_path = dir.path().join("v.sock");
        let sock_path_str = sock_path.to_str().unwrap().to_string();

        let listener = tokio::net::UnixListener::bind(&sock_path).unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = tokio::io::BufReader::new(&mut stream);
            let mut request = String::new();
            reader.read_line(&mut request).await.unwrap();
            assert_eq!(request, "CONNECT 1024\n");

            stream.write_all(b"OK 1024\n").await.unwrap();
            stream.write_all(b"hello from guest").await.unwrap();
        });

        let connector = VsockConnector::new(sock_path_str);
        let mut client_stream = connector.connect_to_guest(1024).await.unwrap();

        let mut buf = vec![0u8; 16];
        let n = tokio::io::AsyncReadExt::read(&mut client_stream, &mut buf)
            .await
            .unwrap();
        assert_eq!(&buf[..n], b"hello from guest");
    }

    #[tokio::test]
    async fn test_vsock_handshake_failure() {
        let dir = tempdir().unwrap();
        let sock_path = dir.path().join("v.sock");
        let sock_path_str = sock_path.to_str().unwrap().to_string();

        let listener = tokio::net::UnixListener::bind(&sock_path).unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = tokio::io::BufReader::new(&mut stream);
            let mut request = String::new();
            reader.read_line(&mut request).await.unwrap();

            stream.write_all(b"ERR Port not listening\n").await.unwrap();
        });

        let connector = VsockConnector::new(sock_path_str);
        let res = connector.connect_to_guest(1024).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Handshake rejected"));
    }

    #[tokio::test]
    async fn test_vsock_tls_upgrade() {
        let creds = generate_test_creds().unwrap();
        let dir = tempdir().unwrap();
        let sock_path = dir.path().join("v.sock");
        let sock_path_str = sock_path.to_str().unwrap().to_string();

        let listener = tokio::net::UnixListener::bind(&sock_path).unwrap();

        let server_config = Arc::new(
            create_vsock_server_config(&creds.ca_cert, &creds.server_cert, &creds.server_key)
                .unwrap(),
        );

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = tokio::io::BufReader::new(&mut stream);
            let mut request = String::new();
            reader.read_line(&mut request).await.unwrap();
            assert_eq!(request, "CONNECT 2048\n");

            stream.write_all(b"OK 2048\n").await.unwrap();

            let acceptor = tokio_rustls::TlsAcceptor::from(server_config);
            let mut secure_stream = acceptor.accept(stream).await.unwrap();

            let mut buf = vec![0u8; 16];
            let n = tokio::io::AsyncReadExt::read(&mut secure_stream, &mut buf)
                .await
                .unwrap();
            assert_eq!(&buf[..n], b"client_ping");

            tokio::io::AsyncWriteExt::write_all(&mut secure_stream, b"server_pong")
                .await
                .unwrap();
        });

        let connector = VsockConnector::new(sock_path_str);
        let client_config = Arc::new(
            create_vsock_client_config(&creds.ca_cert, &creds.client_cert, &creds.client_key)
                .unwrap(),
        );

        let mut client_stream = connector
            .connect_to_guest_secure(2048, client_config, "localhost")
            .await
            .unwrap();

        tokio::io::AsyncWriteExt::write_all(&mut client_stream, b"client_ping")
            .await
            .unwrap();

        let mut buf = vec![0u8; 16];
        let n = tokio::io::AsyncReadExt::read(&mut client_stream, &mut buf)
            .await
            .unwrap();
        assert_eq!(&buf[..n], b"server_pong");
    }
}

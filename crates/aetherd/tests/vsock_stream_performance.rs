// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use aether_auth::mtls::test_pki::generate_test_creds;
use aetherd::vsock::{create_vsock_client_config, create_vsock_server_config, VsockConnector};
use std::sync::Arc;
use std::time::Instant;
use tempfile::tempdir;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_vsock_stream_performance() -> Result<(), Box<dyn std::error::Error>> {
    let creds = generate_test_creds()?;
    let dir = tempdir()?;
    let sock_path = dir.path().join("perf.sock");
    let sock_path_str = sock_path.to_str().unwrap().to_string();

    let listener = tokio::net::UnixListener::bind(&sock_path)?;

    let server_config = Arc::new(create_vsock_server_config(
        &creds.ca_cert,
        &creds.server_cert,
        &creds.server_key,
    )?);

    // 5 MB payload size to perform a robust transfer speed verification
    let data_size = 5 * 1024 * 1024;
    let transmit_data = vec![0u8; data_size];

    // Spawn mock guest UDS listener
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut reader = tokio::io::BufReader::new(&mut stream);
        let mut request = String::new();
        reader.read_line(&mut request).await.unwrap();
        assert_eq!(request, "CONNECT 4096\n");

        stream.write_all(b"OK 4096\n").await.unwrap();

        let acceptor = tokio_rustls::TlsAcceptor::from(server_config);
        let mut secure_stream = acceptor.accept(stream).await.unwrap();

        let mut received = 0;
        let mut buf = vec![0u8; 65536];
        while received < data_size {
            let n = secure_stream.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            received += n;
        }

        // Return confirmation signal
        secure_stream.write_all(b"DONE").await.unwrap();
    });

    let connector = VsockConnector::new(sock_path_str);
    let client_config = Arc::new(create_vsock_client_config(
        &creds.ca_cert,
        &creds.client_cert,
        &creds.client_key,
    )?);

    let mut client_stream = connector
        .connect_to_guest_secure(4096, client_config, "localhost")
        .await?;

    let start_time = Instant::now();

    // Stream the data chunks
    let mut sent = 0;
    let chunk_size = 65536;
    while sent < data_size {
        let end = (sent + chunk_size).min(data_size);
        client_stream.write_all(&transmit_data[sent..end]).await?;
        sent = end;
    }
    client_stream.flush().await?;

    let mut response = vec![0u8; 4];
    client_stream.read_exact(&mut response).await?;
    assert_eq!(&response, b"DONE");

    let duration = start_time.elapsed();
    let speed_mb_s = (data_size as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();
    println!("Transmitted {} bytes in {:?}", data_size, duration);
    println!("Throughput speed: {:.2} MB/s", speed_mb_s);

    // Verify throughput meets the NFR-3.2.2 requirement of >= 100MB/s
    assert!(
        speed_mb_s >= 100.0,
        "Throughput was {:.2} MB/s, lower than 100MB/s limit",
        speed_mb_s
    );

    handle.await?;

    Ok(())
}

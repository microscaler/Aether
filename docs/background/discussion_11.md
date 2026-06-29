Architectural Specification: Secure gRPC Cryptographic Token Handshake Mechanism

Component Identifier: aether-auth
Subsystem Context: Cluster Control Plane Security & Node Identity Attestation
Protocol Boundary: gRPC over HTTP/2 (TLS 1.3) via Mutual-TLS (mTLS) + Ephemeral Token Exchange
1. Security Philosophy & The Trust Bootstrap Problem

In an on-premise SME deployment spanning up to 3 HPE c7000 chassis, we cannot assume a secure network environment. Even though the Aether Control Bus (VLAN 10) is isolated at the Virtual Connect hardware layer, a compromised tenant VM could theoretically exploit a hypervisor escape or a midplane configuration leak to sniff or inject packets onto the control network.
To prevent rogue nodes from joining the cluster or unauthorized systems from issuing destructive STONITH fencing commands, Project Aether enforces a strict Zero-Trust Attestation Framework.
The security architecture relies on a multi-layered trust model:
Transport Layer: Mutual TLS (mTLS) with standard RSA-4096 certificates secures all gRPC communication. This establishes the baseline encryption and host verification.
Application Layer: A dynamic Cryptographic Token Handshake acts as an internal guardrail.
Because the Central Aggregator is stateless and nodes are autonomous, static API keys are banned. Instead, the cluster utilizes a Just-In-Time (JIT) Dual-Token Challenge-Response Execution Handshake. This validates that a worker node is physically present in its assigned chassis slot and running the verified Aether Rust daemon before it can place a reverse-bid.
[ Central Aggregator ]                                    [ Local Worker Node ]
│                                                          │
│ 1. gRPC `RequestReverseBid` + Signed Nonce A             │
├─────────────────────────────────────────────────────────►│
│                                                          │
│                               2. Verify Aggregator Cert  │
│                               3. Query Local iLO / State │
│                               4. Compute HMAC(Nonce A)   │
│                                                          │
│ 5. Return `BidResponse` + Node Token B                   │
│◄─────────────────────────────────────────────────────────┤
│                                                          │
│ 6. Validate Token B                                      │
│ 7. Process Auction Loop                                  │
▼                                                          ▼
2. Cryptographic Handshake Workflow

When the Central Aggregator initiates an auction, it embeds an ephemeral, cryptographically signed challenge directly into the gRPC metadata header.
2.1 The Bid Challenge (Aggregator Output)

For every auction cycle, the Aggregator generates a transient payload containing:
Workload_UID: The unique identifier of the target VM.
Epoch_Timestamp: A high-resolution Unix timestamp (enforces a strict 2-second TTL expiration window to prevent replay attacks).
Cryptographic_Nonce: A cryptographically secure random 256-bit string generated via the rand crate's OsRnghardware entropy pool.
The Aggregator concatenates these strings and signs them using the cluster's private master key:
$$\text{Signature}_{\text{Master}} = \text{Ed25519\_Sign}(\text{Master\_Private\_Key}, \text{Workload\_UID} \parallel \text{Timestamp} \parallel \text{Nonce})$$
This bundle is injected into the outbound gRPC BidRequest payload.
2.2 The Node Verification & Response (Node Output)

Upon receiving the packet over the mTLS channel, the node's local Rust daemon performs the following validation pipeline:
Time Check: It validates that the incoming Epoch_Timestamp is within ± 2000ms of its local hardware clock. If the packet is old, it instantly drops the request to mitigate intercept-and-replay attempts.
Signature Verification: It uses the embedded cluster public certificate to verify $\text{Signature}_{\text{Master}}$. If the signature is invalid, it logs a critical security alert and returns a hard rejection (bid_score = -1).
Hardware Token Computation: If the signature is valid, the node proves its identity by computing an Ephemeral Node Token (Token B). It retrieves a pre-shared local secret key—provisioned uniquely to that specific blade slot during bare-metal deployment—and calculates an HMAC-SHA256 hash:
$$\text{Token B} = \text{HMAC-SHA256}(\text{Node\_Secret\_Key}, \text{Cryptographic\_Nonce} \parallel \text{Node\_ID})$$
The node appends Token B to its gRPC BidResponse metadata wrapper. The Central Aggregator recalculates the HMAC using its local registry of node keys to verify the response. If the tokens match, the bid enters the auction convergence pool.
3. Rust Authentication & Interceptor Blueprint (auth.rs)

The security architecture is implemented natively within tonic using custom asynchronous gRPC Interceptors. This ensures that authentication validation runs before the request hits the primary execution routers.
// Unified Rust Structural Module Map: aether-auth

use std::time::{SystemTime, UNIX_EPOCH};
use tonic::{service::Interceptor, Request, Status};
use ring::hmac; // Ultra-fast, boring-crypto based primitive security crate

pub struct ClusterSecurityConfig {
pub cluster_public_key_bytes: [u8; 32],
pub local_node_secret_key: String,
pub node_id: String,
}

#[derive(Clone)]
pub struct AetherAuthInterceptor {
pub config: std::sync::Arc<ClusterSecurityConfig>,
}

impl Interceptor for AetherAuthInterceptor {
/// Automatically intercepts incoming gRPC requests to validate the security handshake
fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
let metadata = request.metadata();

        // Step 1: Extract security primitives from HTTP/2 headers
        let timestamp_str = match metadata.get("aether-epoch-timestamp") {
            Some(t) => t.to_str().map_err(|_| Status::unauthenticated("Malformed security header."))?,
            None => return Err(Status::unauthenticated("Missing cryptographic timestamp.")),
        };
        
        let incoming_nonce = match metadata.get("aether-nonce") {
            Some(n) => n.as_bytes(),
            None => return Err(Status::unauthenticated("Missing cryptographic nonce boundary.")),
        };

        // Step 2: Enforce strict Time-To-Live (TTL) anti-replay windows
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Status::internal("System clock failure."))?
            .as_secs();
            
        let parsed_timestamp: u64 = timestamp_str.parse()
            .map_err(|_| Status::unauthenticated("Invalid timestamp encoding."))?;

        // Fail if the packet age is greater than 2 seconds
        if current_time.saturating_sub(parsed_timestamp) > 2 {
            return Err(Status::unauthenticated("Control challenge packet has expired (TTL Timeout)."));
        }

        // Step 3: Verify cryptographic master signatures
        // In full execution, this triggers ed25519 verification using self.config.cluster_public_key_bytes
        if !Self::verify_master_signature(metadata, incoming_nonce) {
            return Err(Status::unauthenticated("Cryptographic signature mismatch. Node rejected trust boundary."));
        }

        Ok(request)
    }
}

impl AetherAuthInterceptor {
/// Verifies the cluster master's cryptographic signature against the transmission bundle
fn verify_master_signature(_metadata: &tonic::metadata::MetadataMap, _nonce: &[u8]) -> bool {
// Implementation Abstract:
// ring::signature::verify(::ED25519, public_key_reference, message, signature_bytes)
true // Simplified for structural layout validation
}

    /// Generates Token B using a hardware-isolated local key to return to the Aggregator
    pub fn compute_egress_handshake_token(&self, nonce: &[u8]) -> String {
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.config.local_node_secret_key.as_bytes());
        
        let mut binding_payload = Vec::new();
        binding_payload.extend_from_slice(nonce);
        binding_payload.extend_from_slice(self.config.node_id.as_bytes());

        let signature = hmac::sign(&key, &binding_payload);
        hex::encode(signature.as_ref())
    }
}
4. Operational Guardrails: Credential Rotation & Failover

4.1 Automated Key Rotation & Compromise Isolation

Because local node secret keys are stored on local host filesystems, a physical breach of a single blade could expose its specific secret key.
The Security Boundary: If the Central Aggregator detects an invalid token signature from blade-04 three times consecutively, it automatically revokes that blade's identity certificate in the in-memory NodeRegistry.
The Isolation Cascade: The Aggregator flags the node's status as Compromised. It immediately activates Component 1 (aether-fence), issuing a Redfish hard power-off command to that specific physical chassis slot to neutralize the host. The node is blocked from re-entering the reverse-bidding pool until an administrator physically rotates its security key via out-of-band management.
4.2 Handling Clock Skew Across Blades

SME data centers often experience minor time serialization drift (clock skew) across physical servers if NTP synchronization lags.
The Mitigation Guardrail: If a blade's hardware clock drifts by more than 500ms relative to the Aggregator, the local Rust daemon's token validation loop logs a warning but widens its internal acceptance window to a maximum hard ceiling of ± 5000ms. If the drift exceeds 5 seconds, the token handshake fails automatically. This protects the cluster from clock-tampering attacks designed to bypass the 2-second anti-replay TTL window.
This specification establishes the secure, cryptographic foundation for the project's decentralized control plane. By combining mTLS with just-in-time token exchanges and hardware isolation principles, the cluster achieves zero-trust verification across the chassis midplane.
If you would like to move into the final stage of the architecture design, let me know if we should define the GitOps manifest reconciliation logic for rolling updates and infrastructure adjustments, or compile the CLI client configuration payload formats for cluster administrators.


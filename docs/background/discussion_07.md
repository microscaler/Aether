Architectural Specification: STONITH / iLO Fencing Module

Component Identifier: aether-fence
Subsystem Context: Out-of-Band (OOB) Hardware Enforcement Plane
Target Hardware Compliance: HPE Integrated Lights-Out 5 (iLO 5) via Redfish Scalable API
1. System Philosophy & Fencing Rationale

In a decentralized infrastructure layer where nodes participate in a reverse-bidding marketplace, network isolation is an inevitable failure state. If a physical blade server loses access to the control midplane but remains operational, it may continue executing guest virtual machine processes that read and write data to shared NVMe-over-Fabrics pools or localized disks.
To safely re-allocate resources without causing catastrophic file system splits or data corruption, the Aether Cluster Aggregator enforces Hard Fencing (STONITH - Shoot The Other Node In The Head). No orphan workload configuration may enter a new reverse-bid auction cycle until the physical slot housing the rogue blade has been successfully and verifiably powered down via its independent on-board management processor. [1]
[ Worker Node Fails gRPC Keep-Alives (15s) ]
│
▼
[ Aggregator Suspends Workload Auctions ]
│
▼
┌───────────────────────────────────┐
│  Invoke `aether-fence` Subsystem  │
└─────────────────┬─────────────────┘
│
▼
┌───────────────────────────────────┐
│ Step 1: Open Redfish HTTP Session │
│ POST /redfish/v1/SessionService   │
└─────────────────┬─────────────────┘
│
┌───────────────┴───────────────┐
▼                               ▼
[ Success ]                     [ Failure ]
│                               │
▼                               ▼
┌───────────────────┐           ┌───────────────────┐
│Step 2: Force Power│           │ Fallback: Query   │
│Off Directive      │           │ Chassis Onboard   │
│POST .../Actions/  │           │ Administrator (OA)│
│ComputerSystem.Reset │         │ CLI over SSH      │
└─────────┬─────────┘           └─────────┬─────────┘
│                               │
▼                               ▼
┌───────────────────┐           ┌───────────────────┐
│Step 3: Verification│          │ Hard Truncation   │
│Loop GET /Systems/1│           │ Execution Loop    │
└─────────┬─────────┘           └─────────┬─────────┘
│                               │
┌───────┴───────┐                       │
▼               ▼                       │
[State: Off]   [State: Alive]               │
│               │                       │
▼               ▼                       ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│Safe to        │ │Raise Panic:   │ │Panic: Fatal   │
│Auction Pool   │ │Fencing Failed │ │Chassis Fault  │
└───────────────┘ └───────────────┘ └───────────────┘
2. API Endpoint Protocol Maps & Payload Contracts

The fencing engine bypasses legacy IPMI commands, utilizing raw, type-safe HTTPS operations against the iLO 5 Redfish REST ecosystem.
2.1 Authentication & Session Initiation

HTTP Method: POST
URI Paths: https://<ilo_ip_or_fqdn>/redfish/v1/SessionService/Sessions
Request Payload Format:
{
"UserName": "aether_fence_agent",
"Password": "VaultEnforcedSecureHardwareTokenPassword"
}
Success Output Criteria: HTTP Status 201 Created. The engine must extract the X-Auth-Token value from the response headers and save it as an ephemeral state variable for consecutive pipeline commands.
2.2 Hard Power Disruption

HTTP Method: POST
URI Paths: https://<ilo_ip_or_fqdn>/redfish/v1/Systems/1/Actions/ComputerSystem.Reset
Required Authentication Header: X-Auth-Token: <token_string>
Request Payload Format:
{
"ResetType": "ForceOff"
}
Success Output Criteria: HTTP Status 200 OK or 204 No Content.
2.3 Real-Time State Verification Loop

HTTP Method: GET
URI Paths: https://<ilo_ip_or_fqdn>/redfish/v1/Systems/1/
Success Output Criteria: HTTP Status 200 OK. The JSON structure must parse down to evaluate the node's power footprint properties:
{
"PowerState": "Off"
}
3. Rust Architectural Type Blueprint (fence.rs)

The fencing module is modeled as a zero-allocating state machine utilizing asynchronous network traits. This blueprint dictates the code boundaries, explicit typing structures, and dependency graphs before writing functional implementation logic.
// Unified Rust Structural Module Map: aether-fence

use std::time::Duration;

/// Concrete states representing the target physical blade power conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BladePowerState {
On,
Off,
PoweringOn,
PoweringOff,
Unknown,
}

/// Enforced hardware action targets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceAction {
/// Absolute power interruption (ForceOff)
ForceOff,
/// Hard execution loop truncation followed by boot (ForceRestart)
ForceReset,
}

/// Execution output states handed back to the Cluster Aggregator Core
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FenceOutcome {
/// Blade state is verified as completely isolated and unpowered
Success,
/// Redfish endpoint responded, but PowerState refused to shift to "Off"
FencingFailedActionIncomplete,
/// iLO board network layer dropped frames or timed out completely
IloNetworkUnreachable,
/// Session credentials rejected by the on-board processor
AuthenticationDenied,
}

/// Structured profile containing addressing definitions for target blade hardware
pub struct IloCredentials {
pub endpoint_ip: String,
pub username: String,
pub password_secret: String,
pub session_token: Option<String>,
}

/// The programmatic context wrapping the stateful out-of-band fencing execution loops
pub struct AetherFenceAgent {
pub credentials: IloCredentials,
pub timeout_window: Duration,
pub verification_retries: u32,
pub backoff_delay: Duration,
}

impl AetherFenceAgent {
/// Prepares runtime constraints for targeted fence automation
pub fn new(ip: String, user: String, secret: String) -> Self {
Self {
credentials: IloCredentials {
endpoint_ip: ip,
username: user,
password_secret: secret,
session_token: None,
},
timeout_window: Duration::from_millis(3000),
verification_retries: 5,
backoff_delay: Duration::from_millis(1000),
}
}

    /// Authenticates to the target iLO 5 API and registers an ephemeral session token context
    pub async fn establish_session(&mut self) -> Result<(), FenceOutcome> {
        // Implementation Path:
        // 1. Initialize client pools using secure TLS configurations (rustls/native-tls).
        // 2. Dispatch serialization payload mapping to Redfish session endpoint parameters.
        // 3. Extract string matching 'X-Auth-Token' value from header maps.
        // 4. Update self.credentials.session_token = Some(token).
        todo!()
    }

    /// Primary execution vector called by the Central gRPC Cluster Aggregator Failover Loop
    pub async fn trigger_fence_enforcement(&mut self, action: FenceAction) -> FenceOutcome {
        // Implementation Path:
        // 1. Call self.establish_session().await and check results.
        // 2. Dispatch a HTTP POST command carrying {"ResetType": "ForceOff"} to the target machine context.
        // 3. Evaluate response structures. If valid, immediately cascade down to verify state loop boundaries.
        // 4. Invoke self.verify_hardware_halt_state().await inside an optimization execution wrapper.
        todo!()
    }

    /// Asynchronous verification engine looping until target state is confirmed or max retries are exhausted
    async fn verify_hardware_halt_state(&self) -> Result<BladePowerState, FenceOutcome> {
        // Implementation Path:
        // 1. Initialize an integer counter tracking current loop iteration.
        // 2. Execute an HTTP GET query targeting Redfish machine descriptors.
        // 3. Deserialize JSON string payload extracting raw structural value inside the "PowerState" boundary keys.
        // 4. If value maps to "Off", break loop and return Ok(BladePowerState::Off).
        // 5. If value maps to "On" and loop iteration < self.verification_retries, sleep for self.backoff_delay and repeat.
        // 6. If loop limit is breached without reaching target state, return Err(FenceOutcome::FencingFailedActionIncomplete).
        todo!()
    }
}
4. Operational Fallback Engineering & Error Isolation

4.1 Redfish API Timeout & Midplane Network Drops

If an entire blade's management backplane fails, its specific iLO IP address will become completely unreachable over the network, returning a connection timeout error.
The Fallback Protocol: If trigger_fence_enforcement maps structurally to FenceOutcome::IloNetworkUnreachable, the agent drops out of Redfish execution boundaries and falls back to opening an automated SSH channel directly to the parent HPE c7000 Onboard Administrator (OA) CLI.
The OA Power Override Pipeline: The agent authenticates to the chassis OA management gateway and fires native hardware control override macros down the midplane serial traces directly to the target physical slot:
power off server 16 force
4.2 Structural Guardrails & Crash Isolation

The fencing framework strictly enforces asynchronous boundaries inside Tokio. If an out-of-band endpoint lags or suffers from packet loss, the AetherFenceAgent will wrap the whole function loop inside a hard tokio::time::timeout block. This prevents a misconfigured or hanging physical server blade from locking up the central aggregator's asynchronous message queues or blocking healthy blade engines from operating.
With this design document in place, we have established the structural blueprints for the out-of-band power fencing planes. Let me know if you would like to explore the central gRPC cluster aggregator's tie-breaking decision engine, or if you want to detail how the GitOps file synchronization watcher maps manifests to the auction queues.

[1] https://en.wikipedia.org

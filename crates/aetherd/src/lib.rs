// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![allow(missing_docs)]

pub mod bidder;
pub mod cloud_init;
pub mod hypervisor;
pub mod migration;
pub mod network;
pub mod storage;
pub mod telemetry;
pub mod vsock;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use aether_auth::proto::aether_node_server::AetherNode;
use aether_auth::proto::{
    BidRequest, BidResponse, ExecuteVmRequest, ExecuteVmResponse, ListVMsRequest, ListVMsResponse,
    TeardownVmRequest, TeardownVmResponse, VmDetails,
};
use aether_auth::token::TokenManager;

use crate::bidder::Bidder;
use crate::hypervisor::Hypervisor;
use crate::telemetry::TelemetryCollector;

/// Struct tracking details of an actively running VM.
pub struct ActiveVm {
    /// Metadata about the VM.
    pub details: VmDetails,
    /// The hypervisor instance managing the process.
    pub hypervisor: Box<dyn Hypervisor>,
    /// Optional iSCSI IQN if this VM uses a network volume.
    pub iscsi_iqn: Option<String>,
    /// Cloud-Init ISO handle kept in memory to maintain tempfile scope.
    pub _iso: crate::cloud_init::CloudInitIso,
}

/// gRPC service implementation for Aether Node Daemon.
pub struct AetherNodeImpl {
    /// Unique identifier for the node.
    pub node_id: String,
    /// Allocated pool: "COMPUTE" or "INFRA".
    pub pool: String,
    /// Authenticator for token verification.
    pub token_manager: Arc<TokenManager>,
    /// Telemetry collector to fetch host metrics.
    pub telemetry_collector: Arc<TelemetryCollector>,
    /// Bidder calculator to compute auction scores.
    pub bidder: Arc<Bidder>,
    /// Map of active VMs currently running on this node daemon.
    pub active_vms: Arc<Mutex<HashMap<String, ActiveVm>>>,
    /// Manager for VM migrations.
    pub migration_manager: Arc<dyn crate::migration::MigrationManager>,
    /// Manager for iSCSI sessions.
    pub iscsi_manager: Arc<dyn crate::storage::iscsi::IscsiManager>,
}

impl AetherNodeImpl {
    /// Creates a new instance of AetherNodeImpl.
    pub fn new(
        node_id: String,
        pool: String,
        token_manager: Arc<TokenManager>,
        telemetry_collector: Arc<TelemetryCollector>,
        bidder: Arc<Bidder>,
        migration_manager: Arc<dyn crate::migration::MigrationManager>,
        iscsi_manager: Arc<dyn crate::storage::iscsi::IscsiManager>,
    ) -> Self {
        Self {
            node_id,
            pool,
            token_manager,
            telemetry_collector,
            bidder,
            active_vms: Arc::new(Mutex::new(HashMap::new())),
            migration_manager,
            iscsi_manager,
        }
    }
}

#[tonic::async_trait]
impl AetherNode for AetherNodeImpl {
    async fn request_reverse_bid(
        &self,
        request: Request<BidRequest>,
    ) -> Result<Response<BidResponse>, Status> {
        let req = request.into_inner();
        let migration_count = self.migration_manager.get_active_migration_count().await;
        let metrics = self.telemetry_collector.collect(migration_count);
        let score = self.bidder.calculate_bid(
            &metrics,
            req.cpu_request,
            req.memory_request_bytes,
            req.disk_request_bytes,
        );

        Ok(Response::new(BidResponse {
            node_id: self.node_id.clone(),
            score,
        }))
    }

    async fn execute_vm(
        &self,
        request: Request<ExecuteVmRequest>,
    ) -> Result<Response<ExecuteVmResponse>, Status> {
        let req = request.into_inner();
        self.token_manager
            .validate_token(&req.token, &self.node_id)
            .map_err(Status::unauthenticated)?;

        let uuid = req.workload_uuid.clone();

        // Compile cloud-init ISO in memory (tmpfs /dev/shm)
        let ci_config = crate::cloud_init::CloudInitConfig {
            instance_id: req.workload_uuid.clone(),
            hostname: req.name.clone(),
            user_data: "#cloud-config\n".to_string(),
        };
        let builder = crate::cloud_init::CloudInitIsoBuilder::new(ci_config);
        let iso = builder.build_iso().await.map_err(Status::internal)?;
        let _iso_path = iso.path().to_str().unwrap_or("").to_string();
        let mut iscsi_iqn = None;

        let hypervisor: Box<dyn Hypervisor> = if req.cpu_limit < 4 {
            // Blade node: deploy lightweight Firecracker microVM
            let fc_config = crate::hypervisor::firecracker::FirecrackerConfig {
                boot_source: crate::hypervisor::firecracker::BootSource {
                    kernel_image_path: "/var/lib/aether/vmlinux".to_string(),
                    boot_args: "console=ttyS0 reboot=k panic=1 pci=off".to_string(),
                },
                drives: vec![crate::hypervisor::firecracker::Drive {
                    drive_id: "rootfs".to_string(),
                    path_on_host: req.image_uri.clone(),
                    is_root_device: true,
                    is_read_only: false,
                }],
                machine_config: crate::hypervisor::firecracker::MachineConfig {
                    vcpu_count: req.cpu_limit as u32,
                    mem_size_mib: (req.memory_limit_bytes / (1024 * 1024)) as u32,
                    smt: Some(false),
                },
                network_interfaces: Vec::new(),
            };

            let bin_path = if std::path::Path::new("/usr/bin/firecracker").exists() {
                "/usr/bin/firecracker"
            } else {
                "sleep"
            }
            .to_string();

            let log_path = std::env::temp_dir()
                .join(format!("fc-{}.log", uuid))
                .to_str()
                .unwrap_or("")
                .to_string();

            let config_path = std::env::temp_dir()
                .join(format!("fc-{}.json", uuid))
                .to_str()
                .unwrap_or("")
                .to_string();

            let mut fc = crate::hypervisor::firecracker::FirecrackerHypervisor::new(
                uuid.clone(),
                bin_path.clone(),
                config_path,
                log_path,
                fc_config,
            );
            if bin_path == "sleep" {
                fc.extra_args = vec!["1000".to_string()];
            }

            Box::new(fc)
        } else {
            // Check for iSCSI image URI
            let disk_image_path = if req.image_uri.starts_with("iscsi://") {
                let stripped = req.image_uri.strip_prefix("iscsi://").unwrap_or("");
                let mut parts = stripped.splitn(2, '/');
                let portal_ip = parts.next().unwrap_or("");
                let iqn = parts.next().unwrap_or("");
                iscsi_iqn = Some(iqn.to_string());

                self.iscsi_manager
                    .login_target(portal_ip, iqn)
                    .await
                    .map_err(|e| Status::internal(format!("iSCSI login failed: {}", e)))?
            } else {
                req.image_uri.clone()
            };

            // Infra node: deploy full QEMU VM
            let qemu_config = crate::hypervisor::qemu::QemuConfig {
                vcpu_count: req.cpu_limit as u32,
                mem_size_mib: (req.memory_limit_bytes / (1024 * 1024)) as u32,
                disk_image_path,
                qmp_socket_path: std::env::temp_dir()
                    .join(format!("qmp-{}.sock", uuid))
                    .to_str()
                    .unwrap_or("")
                    .to_string(),
                host_tap_device: None,
            };

            let bin_path = if std::path::Path::new("/usr/bin/qemu-system-x86_64").exists() {
                "/usr/bin/qemu-system-x86_64"
            } else {
                "sleep"
            }
            .to_string();

            let log_path = std::env::temp_dir()
                .join(format!("qemu-{}.log", uuid))
                .to_str()
                .unwrap_or("")
                .to_string();

            let mut qemu = crate::hypervisor::qemu::QemuHypervisor::new(
                uuid.clone(),
                bin_path.clone(),
                log_path,
                qemu_config,
            );
            if bin_path == "sleep" {
                qemu.extra_args = vec!["1000".to_string()];
            }

            Box::new(qemu)
        };

        hypervisor
            .spawn()
            .await
            .map_err(|e| Status::internal(format!("Hypervisor spawn failed: {}", e)))?;

        // Register with migration manager if QMP is available
        if let Some(qmp_path) = hypervisor.get_qmp_socket_path() {
            let _ = self.migration_manager.register_vm(&uuid, &qmp_path).await;
        }

        let details = VmDetails {
            uuid: uuid.clone(),
            name: req.name.clone(),
            state: "RUNNING".to_string(),
            ip_address: "192.168.1.100".to_string(),
            mac_address: "52:54:00:12:34:56".to_string(),
        };

        let mut active = self.active_vms.lock().await;
        active.insert(
            uuid,
            ActiveVm {
                details: details.clone(),
                hypervisor,
                _iso: iso,
                iscsi_iqn,
            },
        );

        Ok(Response::new(ExecuteVmResponse {
            success: true,
            ip_address: details.ip_address,
            mac_address: details.mac_address,
            error_message: String::new(),
        }))
    }

    async fn teardown_vm(
        &self,
        request: Request<TeardownVmRequest>,
    ) -> Result<Response<TeardownVmResponse>, Status> {
        let req = request.into_inner();
        self.token_manager
            .validate_token(&req.token, &self.node_id)
            .map_err(Status::unauthenticated)?;

        let mut active = self.active_vms.lock().await;
        if let Some(vm) = active.remove(&req.workload_uuid) {
            vm.hypervisor
                .stop()
                .await
                .map_err(|e| Status::internal(format!("Hypervisor stop failed: {}", e)))?;

            // Unregister from migration manager
            let _ = self
                .migration_manager
                .unregister_vm(&req.workload_uuid)
                .await;

            // iSCSI logout if applicable
            if let Some(iqn) = vm.iscsi_iqn {
                let _ = self.iscsi_manager.logout_target(&iqn).await;
            }

            Ok(Response::new(TeardownVmResponse {
                success: true,
                error_message: String::new(),
            }))
        } else {
            Err(Status::not_found("VM not found"))
        }
    }

    async fn list_v_ms(
        &self,
        _request: Request<ListVMsRequest>,
    ) -> Result<Response<ListVMsResponse>, Status> {
        let mut active = self.active_vms.lock().await;
        let mut vms = Vec::new();
        for vm in active.values_mut() {
            if let Ok(status) = vm.hypervisor.query_status().await {
                vm.details.state = status;
            }
            vms.push(vm.details.clone());
        }
        Ok(Response::new(ListVMsResponse { vms }))
    }

    async fn prepare_migration(
        &self,
        request: Request<aether_auth::proto::PrepareMigrationRequest>,
    ) -> Result<Response<aether_auth::proto::PrepareMigrationResponse>, Status> {
        let req = request.into_inner();
        self.token_manager
            .validate_token(&req.token, &self.node_id)
            .map_err(Status::unauthenticated)?;

        self.migration_manager
            .prepare_incoming(&req.workload_uuid, req.port as u16, req.use_tls)
            .await
            .map_err(Status::internal)?;

        Ok(Response::new(
            aether_auth::proto::PrepareMigrationResponse {
                success: true,
                error_message: String::new(),
            },
        ))
    }

    async fn start_migration(
        &self,
        request: Request<aether_auth::proto::StartMigrationRequest>,
    ) -> Result<Response<aether_auth::proto::StartMigrationResponse>, Status> {
        let req = request.into_inner();
        self.token_manager
            .validate_token(&req.token, &self.node_id)
            .map_err(Status::unauthenticated)?;

        let params = crate::migration::MigrationParams {
            destination_node: "unknown".to_string(), // Node ID not in proto yet
            destination_ip: req.destination_ip,
            port: req.port as u16,
            use_tls: req.use_tls,
            max_bandwidth: req.max_bandwidth,
        };

        self.migration_manager
            .start_migration(&req.workload_uuid, params)
            .await
            .map_err(Status::internal)?;

        Ok(Response::new(aether_auth::proto::StartMigrationResponse {
            success: true,
            error_message: String::new(),
        }))
    }
}

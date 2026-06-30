use super::BridgeManager;
use async_trait::async_trait;
use std::io;

#[derive(Default)]
pub struct MockBridgeManager;

impl MockBridgeManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BridgeManager for MockBridgeManager {
    async fn create_tenant_bridge(&self, vlan_id: u16) -> io::Result<String> {
        log::info!(
            "MockBridgeManager: Creating tenant bridge for VLAN {}",
            vlan_id
        );
        Ok(format!("br-tenant-{}", vlan_id))
    }

    async fn create_tap_device(&self, tap_name: &str, bridge_name: &str) -> io::Result<()> {
        log::info!(
            "MockBridgeManager: Creating TAP device {} on bridge {}",
            tap_name,
            bridge_name
        );
        Ok(())
    }

    async fn apply_mac_anti_spoofing(&self, tap_name: &str, allowed_mac: &str) -> io::Result<()> {
        log::info!(
            "MockBridgeManager: Applying MAC anti-spoofing on TAP {} for MAC {}",
            tap_name,
            allowed_mac
        );
        Ok(())
    }

    async fn teardown_tenant_network(&self, vlan_id: u16, tap_name: &str) -> io::Result<()> {
        log::info!(
            "MockBridgeManager: Tearing down network for VLAN {} and TAP {}",
            vlan_id,
            tap_name
        );
        Ok(())
    }
}

#[cfg(all(target_os = "linux", not(tarpaulin)))]
async fn find_link_by_name(handle: &rtnetlink::Handle, name: &str) -> io::Result<Option<u32>> {
    use futures::stream::TryStreamExt;
    let mut links = handle.link().get().match_name(name.to_string()).execute();
    match links.try_next().await {
        Ok(Some(link)) => Ok(Some(link.header.index)),
        Ok(None) => Ok(None),
        Err(rtnetlink::Error::NetlinkError(err_msg))
            if err_msg.code.map(|c| c.get()) == Some(-19) =>
        {
            // ENODEV: device does not exist
            Ok(None)
        }
        Err(e) => Err(io::Error::other(e)),
    }
}

#[cfg(all(target_os = "linux", not(tarpaulin)))]
#[derive(Default)]
pub struct RealBridgeManager;

#[cfg(all(target_os = "linux", not(tarpaulin)))]
impl RealBridgeManager {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(all(target_os = "linux", not(tarpaulin)))]
#[async_trait]
impl BridgeManager for RealBridgeManager {
    async fn create_tenant_bridge(&self, vlan_id: u16) -> io::Result<String> {
        use rtnetlink::new_connection;

        let name = format!("br-tenant-{}", vlan_id);
        let (connection, handle, _) = new_connection().map_err(io::Error::other)?;
        tokio::spawn(connection);

        if let Some(index) = find_link_by_name(&handle, &name).await? {
            handle
                .link()
                .set(index)
                .up()
                .execute()
                .await
                .map_err(io::Error::other)?;
            return Ok(name);
        }

        handle
            .link()
            .add()
            .bridge(name.clone())
            .execute()
            .await
            .map_err(io::Error::other)?;

        let index = find_link_by_name(&handle, &name).await?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "bridge not found after creation")
        })?;

        handle
            .link()
            .set(index)
            .up()
            .execute()
            .await
            .map_err(io::Error::other)?;

        Ok(name)
    }

    async fn create_tap_device(&self, tap_name: &str, bridge_name: &str) -> io::Result<()> {
        use rtnetlink::new_connection;
        use tun_rs::{DeviceBuilder, Layer};

        // 1. Create persistent TAP device using tun-rs
        let dev = DeviceBuilder::new()
            .name(tap_name)
            .layer(Layer::L2)
            .build_sync()
            .map_err(io::Error::other)?;

        dev.persist().map_err(io::Error::other)?;

        // 2. Bind the TAP to the bridge using rtnetlink
        let (connection, handle, _) = new_connection().map_err(io::Error::other)?;
        tokio::spawn(connection);

        let bridge_index = find_link_by_name(&handle, bridge_name)
            .await?
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("bridge {} not found", bridge_name),
                )
            })?;

        let tap_index = find_link_by_name(&handle, tap_name).await?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("tap {} not found", tap_name),
            )
        })?;

        handle
            .link()
            .set(tap_index)
            .controller(bridge_index)
            .execute()
            .await
            .map_err(io::Error::other)?;
        handle
            .link()
            .set(tap_index)
            .up()
            .execute()
            .await
            .map_err(io::Error::other)?;

        Ok(())
    }

    async fn apply_mac_anti_spoofing(&self, tap_name: &str, allowed_mac: &str) -> io::Result<()> {
        use rustables::expr::{Cmp, CmpOp, HighLevelPayload, LLHeaderField};
        use rustables::{
            Batch, Chain, ChainPolicy, ChainType, Hook, HookClass, MsgType, ProtocolFamily, Rule,
            Table,
        };

        // Parse MAC address into 6 bytes safely
        let mut mac_bytes = Vec::new();
        for part in allowed_mac.split(':') {
            let b = u8::from_str_radix(part, 16).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid MAC octet: {}", e),
                )
            })?;
            mac_bytes.push(b);
        }
        if mac_bytes.len() != 6 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "MAC address must be 6 octets",
            ));
        }

        let mut batch = Batch::new();

        // 1. Table
        let table = Table::new(ProtocolFamily::Bridge).with_name("aether-filter");
        batch.add(&table, MsgType::Add);

        // 2. Chain
        let chain_name = format!("filter-{}", tap_name);
        let hook = Hook::new(HookClass::PreRouting, 0);
        let chain = Chain::new(&table)
            .with_name(chain_name)
            .with_hook(hook)
            .with_type(ChainType::Filter)
            .with_policy(ChainPolicy::Accept);
        batch.add(&chain, MsgType::Add);

        // 3. Rule
        let mut rule = Rule::new(&chain).map_err(io::Error::other)?;
        rule = rule.iiface(tap_name).map_err(io::Error::other)?;
        rule.add_expr(HighLevelPayload::LinkLayer(LLHeaderField::Saddr).build());
        rule.add_expr(Cmp::new(CmpOp::Neq, mac_bytes));
        rule = rule.drop();
        batch.add(&rule, MsgType::Add);

        // Send ruleset batch
        batch
            .send()
            .map_err(|e| io::Error::other(format!("Failed to apply nftables ruleset: {}", e)))?;

        Ok(())
    }

    async fn teardown_tenant_network(&self, vlan_id: u16, tap_name: &str) -> io::Result<()> {
        use rtnetlink::new_connection;
        use rustables::{Batch, Chain, MsgType, ProtocolFamily, Table};

        // 1. Delete nftables chain (clears and deletes anti-spoofing rules)
        let mut batch = Batch::new();
        let table = Table::new(ProtocolFamily::Bridge).with_name("aether-filter");
        let chain = Chain::new(&table).with_name(format!("filter-{}", tap_name));
        batch.add(&chain, MsgType::Del);
        // Ignore deletion errors if table/chain does not exist
        let _ = batch.send();

        // 2. Delete network interfaces using rtnetlink
        let bridge_name = format!("br-tenant-{}", vlan_id);
        let (connection, handle, _) = new_connection().map_err(io::Error::other)?;
        tokio::spawn(connection);

        // Delete TAP link if it exists
        if let Some(tap_index) = find_link_by_name(&handle, tap_name).await? {
            handle
                .link()
                .del(tap_index)
                .execute()
                .await
                .map_err(io::Error::other)?;
        }

        // Delete Bridge link if it exists
        if let Some(bridge_index) = find_link_by_name(&handle, &bridge_name).await? {
            handle
                .link()
                .del(bridge_index)
                .execute()
                .await
                .map_err(io::Error::other)?;
        }

        Ok(())
    }
}

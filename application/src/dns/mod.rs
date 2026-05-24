use std::path::Path;

use anyhow::Context;
use featherfly_plugin_sdk::PluginEvent;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    cache::DnsRecordRow,
    routes::State,
    utils::plugin_events::{self, DnsDnssecUpdatedPayload, DnsSyncedPayload},
};

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DnsRecord {
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub value: String,
    #[serde(default = "default_ttl")]
    pub ttl: i64,
}

fn default_ttl() -> i64 {
    300
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DsRecord {
    pub key_tag: u16,
    pub algorithm: u8,
    pub digest_type: u8,
    pub digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SyncDnsRequest {
    pub records: Vec<DnsRecord>,
    #[serde(default)]
    pub dnssec_enabled: bool,
    #[serde(default)]
    pub ds_records: Vec<DsRecord>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DnsZoneStatus {
    pub records: Vec<DnsRecord>,
    pub dnssec_enabled: bool,
    pub ds_records: Vec<DsRecord>,
}

pub struct DnsService;

impl DnsService {
    pub async fn sync(
        state: &State,
        site_id: &str,
        req: SyncDnsRequest,
    ) -> Result<DnsZoneStatus, anyhow::Error> {
        let inner = state.config.load();
        let cache = state.cache.as_ref().context("cache unavailable")?;
        cache
            .get_site(site_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("site not found"))?;

        let now = chrono::Utc::now().timestamp();
        let rows: Vec<DnsRecordRow> = req
            .records
            .iter()
            .map(|r| DnsRecordRow {
                id: Uuid::new_v4().to_string(),
                site_id: site_id.to_string(),
                name: r.name.clone(),
                record_type: r.record_type.clone(),
                value: r.value.clone(),
                ttl: r.ttl,
                created_at: now,
            })
            .collect();

        cache.replace_dns(site_id, &rows).await?;
        write_zone_file(
            &inner.system.data,
            site_id,
            &rows,
            req.dnssec_enabled,
            &req.ds_records,
        )?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::DnsSynced,
            &DnsSyncedPayload {
                site_id,
                record_count: rows.len(),
            },
        );

        if req.dnssec_enabled || !req.ds_records.is_empty() {
            plugin_events::emit_state_event(
                state,
                PluginEvent::DnsDnssecUpdated,
                &DnsDnssecUpdatedPayload {
                    site_id,
                    enabled: req.dnssec_enabled,
                    ds_count: req.ds_records.len(),
                },
            );
        }

        Ok(DnsZoneStatus {
            records: rows
                .iter()
                .map(|r| DnsRecord {
                    name: r.name.clone(),
                    record_type: r.record_type.clone(),
                    value: r.value.clone(),
                    ttl: r.ttl,
                })
                .collect(),
            dnssec_enabled: req.dnssec_enabled,
            ds_records: req.ds_records,
        })
    }

    pub async fn list(state: &State, site_id: &str) -> Result<DnsZoneStatus, anyhow::Error> {
        let inner = state.config.load();
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let records: Vec<DnsRecord> = cache
            .list_dns(site_id)
            .await?
            .iter()
            .map(|r| DnsRecord {
                name: r.name.clone(),
                record_type: r.record_type.clone(),
                value: r.value.clone(),
                ttl: r.ttl,
            })
            .collect();

        let (dnssec_enabled, ds_records) = read_dnssec_meta(&inner.system.data, site_id);

        Ok(DnsZoneStatus {
            records,
            dnssec_enabled,
            ds_records,
        })
    }
}

fn zone_dir(data_dir: &str, site_id: &str) -> std::path::PathBuf {
    Path::new(data_dir).join("dns").join(site_id)
}

fn write_zone_file(
    data_dir: &str,
    site_id: &str,
    rows: &[DnsRecordRow],
    dnssec_enabled: bool,
    ds_records: &[DsRecord],
) -> Result<(), anyhow::Error> {
    let zone_dir = zone_dir(data_dir, site_id);
    std::fs::create_dir_all(&zone_dir)?;
    let mut body = String::from("$ORIGIN .\n");
    if dnssec_enabled {
        body.push_str("; DNSSEC enabled\n");
    }
    for row in rows {
        body.push_str(&format!(
            "{} {} IN {} {}\n",
            row.name, row.ttl, row.record_type, row.value
        ));
    }
    for ds in ds_records {
        body.push_str(&format!(
            "@ 300 IN DS {} {} {} {}\n",
            ds.key_tag, ds.algorithm, ds.digest_type, ds.digest
        ));
    }
    std::fs::write(zone_dir.join("zone.txt"), body)?;

    let meta = serde_json::json!({
        "dnssec_enabled": dnssec_enabled,
        "ds_records": ds_records,
    });
    std::fs::write(
        zone_dir.join("dnssec.json"),
        serde_json::to_vec_pretty(&meta)?,
    )?;
    Ok(())
}

fn read_dnssec_meta(data_dir: &str, site_id: &str) -> (bool, Vec<DsRecord>) {
    let path = zone_dir(data_dir, site_id).join("dnssec.json");
    let Ok(bytes) = std::fs::read(&path) else {
        return (false, Vec::new());
    };
    #[derive(Deserialize)]
    struct Meta {
        #[serde(default)]
        dnssec_enabled: bool,
        #[serde(default)]
        ds_records: Vec<DsRecord>,
    }
    serde_json::from_slice::<Meta>(&bytes)
        .map(|m| (m.dnssec_enabled, m.ds_records))
        .unwrap_or((false, Vec::new()))
}

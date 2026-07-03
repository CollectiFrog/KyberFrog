// SPDX-License-Identifier: AGPL-3.0-or-later

//! mDNS/DNS-SD auto-discovery of the LAN's transmitters (#20).
//!
//! Every KyberFrog announces one `_kyber._tcp.local.` service per *active*
//! transmitter (instance `"<tx>@<hostname>"`, port = the transmitter's
//! control-plane port, TXT = version + transmitter name + source kind) and
//! simultaneously browses the same type, so the viewer form can offer the
//! emitters it heard instead of a hand-typed IP. KyberFrog announces on behalf
//! of its `kycontroller` children — the fork is untouched; a kycontroller
//! started outside KyberFrog is simply not discovered (see IMPROVEMENTS.md #20).
//!
//! Built on `mdns-sd`: pure Rust over raw UDP 5353, no Bonjour/Avahi system
//! dependency (Windows has no reliable native mDNS resolver). The daemon runs
//! its own thread; we re-sync the announced set at every transmitter mutation
//! (the `persist_and_refresh` choke point) and feed browse events into a map
//! the `GET /discovered` handler snapshots. Link-local only, no security —
//! same trusted-LAN assumption as the rest of the app.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use log::{error, info, warn};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;
use shared::Transmitter;

/// The DNS-SD service type every KyberFrog announces and browses.
pub const SERVICE_TYPE: &str = "_kyber._tcp.local.";

/// One emitter instance heard on the LAN, as served to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredInstance {
    /// Transmitter name (the TXT `tx` value, else the instance name).
    pub name: String,
    /// Announcing machine, bare (no `.local.` suffix).
    pub host: String,
    /// Addresses to reach it, IPv4 first (the UI uses the first one).
    pub addrs: Vec<String>,
    /// The transmitter's control-plane port.
    pub port: u16,
    /// KyberFrog version of the announcer (TXT `version`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Source kind (TXT `kind`): `"spout"`, `"screen"` or `"all"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// `true` when the announcer is this very machine (still listed — a local
    /// viewer is legitimate — but the UI can badge it).
    pub is_self: bool,
}

/// What one transmitter announces. Pure data so [`desired_services`] stays
/// testable without touching the network.
#[derive(Debug, Clone, PartialEq)]
struct ServiceSpec {
    /// DNS-SD instance name, collision-safe across machines.
    instance: String,
    port: u16,
    txt: Vec<(String, String)>,
}

impl ServiceSpec {
    /// The fully-qualified service name mdns-sd keys registrations by.
    fn fullname(&self) -> String {
        format!("{}.{SERVICE_TYPE}", self.instance)
    }
}

/// The specs for `transmitters` as announced from `hostname`.
fn desired_services(transmitters: &[Transmitter], hostname: &str) -> Vec<ServiceSpec> {
    transmitters
        .iter()
        .map(|tx| ServiceSpec {
            instance: format!("{}@{hostname}", tx.name),
            port: tx.port,
            txt: vec![
                ("version".to_string(), crate::app::VERSION.to_string()),
                ("tx".to_string(), tx.name.clone()),
                ("kind".to_string(), source_kind(&tx.source).to_string()),
            ],
        })
        .collect()
}

/// The TXT `kind` value (matches the `Source` serde tag).
fn source_kind(source: &shared::Source) -> &'static str {
    match source {
        shared::Source::Spout { .. } => "spout",
        shared::Source::Screen {} => "screen",
        shared::Source::All {} => "all",
    }
}

/// Announcer + browser over one shared mdns-sd daemon. Cheap to clone.
#[derive(Clone)]
pub struct Discovery {
    daemon: ServiceDaemon,
    /// This machine's name, for instance naming and the `is_self` flag.
    hostname: String,
    /// Fullname → port of every service currently registered by us.
    registered: Arc<Mutex<HashMap<String, u16>>>,
    /// Fullname → instance for everything currently heard on the LAN.
    found: Arc<Mutex<HashMap<String, DiscoveredInstance>>>,
}

impl Discovery {
    pub fn new(hostname: String) -> anyhow::Result<Self> {
        let daemon = ServiceDaemon::new()?;
        Ok(Self {
            daemon,
            hostname,
            registered: Arc::new(Mutex::new(HashMap::new())),
            found: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Make the announced set match `transmitters`: register the new ones,
    /// unregister the gone ones (goodbye packets), re-register a port change.
    /// Best-effort — a failed registration is logged, never fatal.
    pub fn sync(&self, transmitters: &[Transmitter]) {
        let desired = desired_services(transmitters, &self.hostname);
        let desired_ports: HashMap<String, u16> =
            desired.iter().map(|s| (s.fullname(), s.port)).collect();

        let Ok(mut registered) = self.registered.lock() else {
            return;
        };

        let stale: Vec<String> = registered
            .iter()
            .filter(|(name, port)| desired_ports.get(*name) != Some(port))
            .map(|(name, _)| name.clone())
            .collect();
        for fullname in stale {
            match self.daemon.unregister(&fullname) {
                Ok(_) => info!("mDNS: unregistered {fullname}"),
                Err(err) => warn!("mDNS: unregistering {fullname}: {err}"),
            }
            registered.remove(&fullname);
        }

        for spec in desired {
            let fullname = spec.fullname();
            if registered.contains_key(&fullname) {
                continue;
            }
            let txt: Vec<(&str, &str)> = spec
                .txt
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let info = ServiceInfo::new(
                SERVICE_TYPE,
                &spec.instance,
                &format!("{}.local.", self.hostname),
                "", // addresses auto-detected below (multi-NIC friendly)
                spec.port,
                &txt[..],
            );
            match info {
                Ok(info) => match self.daemon.register(info.enable_addr_auto()) {
                    Ok(()) => {
                        info!("mDNS: announcing {fullname} on port {}", spec.port);
                        registered.insert(fullname, spec.port);
                    }
                    Err(err) => warn!("mDNS: registering {fullname}: {err}"),
                },
                Err(err) => warn!("mDNS: invalid service info for {fullname}: {err}"),
            }
        }
    }

    /// Start the long-lived browse task feeding [`Self::snapshot`]. Call once.
    pub fn spawn_browser(&self) {
        let receiver = match self.daemon.browse(SERVICE_TYPE) {
            Ok(receiver) => receiver,
            Err(err) => {
                error!("mDNS: browse failed, discovery list will stay empty: {err}");
                return;
            }
        };
        let found = self.found.clone();
        let hostname = self.hostname.clone();
        tokio::spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let instance = resolved_instance(&info, &hostname);
                        info!(
                            "mDNS: found {} at {:?}:{} (self: {})",
                            info.get_fullname(),
                            instance.addrs.first(),
                            instance.port,
                            instance.is_self
                        );
                        if let Ok(mut found) = found.lock() {
                            found.insert(info.get_fullname().to_string(), instance);
                        }
                    }
                    ServiceEvent::ServiceRemoved(_ty, fullname) => {
                        info!("mDNS: lost {fullname}");
                        if let Ok(mut found) = found.lock() {
                            found.remove(&fullname);
                        }
                    }
                    _ => {}
                }
            }
            // The receiver only closes when the daemon shuts down with the app.
            info!("mDNS: browse task stopped");
        });
    }

    /// Everything currently heard, sorted by name then host for a stable UI.
    pub fn snapshot(&self) -> Vec<DiscoveredInstance> {
        let mut list: Vec<DiscoveredInstance> = self
            .found
            .lock()
            .map(|found| found.values().cloned().collect())
            .unwrap_or_default();
        list.sort_by(|a, b| (&a.name, &a.host).cmp(&(&b.name, &b.host)));
        list
    }

    /// Unregister everything (goodbye packets) and stop the daemon. Best-effort
    /// on the way out.
    pub fn shutdown(&self) {
        if let Ok(registered) = self.registered.lock() {
            for fullname in registered.keys() {
                let _ = self.daemon.unregister(fullname);
            }
        }
        let _ = self.daemon.shutdown();
    }
}

/// Map one resolved announcement to the UI shape.
fn resolved_instance(info: &ServiceInfo, our_hostname: &str) -> DiscoveredInstance {
    // "stage-left@REGIE._kyber._tcp.local." → "stage-left@REGIE"
    let instance = info
        .get_fullname()
        .strip_suffix(&format!(".{SERVICE_TYPE}"))
        .unwrap_or(info.get_fullname());
    let host = info
        .get_hostname()
        .trim_end_matches('.')
        .trim_end_matches(".local")
        .trim_end_matches(|c: char| c == '.')
        .to_string();

    // IPv4 first: kyclient reaches the emitter by the first address the UI picks.
    let mut addrs: Vec<std::net::IpAddr> = info.get_addresses().iter().copied().collect();
    addrs.sort_by_key(|addr| !addr.is_ipv4());

    DiscoveredInstance {
        name: info
            .get_property_val_str("tx")
            .map(str::to_string)
            .unwrap_or_else(|| instance.to_string()),
        host: host.clone(),
        addrs: addrs.into_iter().map(|a| a.to_string()).collect(),
        port: info.get_port(),
        version: info.get_property_val_str("version").map(str::to_string),
        kind: info.get_property_val_str("kind").map(str::to_string),
        is_self: host.eq_ignore_ascii_case(our_hostname),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::Source;

    fn tx(name: &str, port: u16, source: Source) -> Transmitter {
        Transmitter {
            name: name.to_string(),
            port,
            source,
        }
    }

    #[test]
    fn desired_services_names_are_collision_safe_across_machines() {
        let txs = vec![
            tx("stage-left", 9000, Source::Spout { sender: "Out A".into() }),
            tx("preview", 9001, Source::Screen {}),
        ];
        let specs = desired_services(&txs, "REGIE");
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].instance, "stage-left@REGIE");
        assert_eq!(specs[0].port, 9000);
        assert_eq!(specs[1].instance, "preview@REGIE");
        // The same transmitter name on another machine yields another instance.
        let other = desired_services(&txs[..1], "SCENE");
        assert_ne!(other[0].instance, specs[0].instance);
    }

    #[test]
    fn desired_services_txt_carries_version_name_and_kind() {
        let specs = desired_services(&[tx("tout-envoyer", 9000, Source::All {})], "REGIE");
        let txt: std::collections::HashMap<_, _> = specs[0]
            .txt
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        assert_eq!(txt["tx"], "tout-envoyer");
        assert_eq!(txt["kind"], "all");
        assert_eq!(txt["version"], crate::app::VERSION);
    }

    #[test]
    fn fullname_appends_service_type() {
        let specs = desired_services(&[tx("a", 1, Source::Screen {})], "H");
        assert_eq!(specs[0].fullname(), "a@H._kyber._tcp.local.");
    }
}

use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicU32;
use std::sync::Mutex as StdMutex;
use std::time::Instant;
use tokio::sync::{Notify, RwLock};

use crate::clash::ClashClient;
use crate::events::AutoSwitch;
use crate::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConnectionState {
    Connected,
    Degraded,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GroupKind {
    Selector,
    UrlTest,
    Fallback,
    LoadBalance,
    Relay,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeView {
    pub name: String,
    pub country: String,
    pub kind: String,
    pub latest_delay: Option<u32>,
    pub is_group: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupView {
    pub name: String,
    pub kind: GroupKind,
    pub now: Option<String>,
    pub now_country: Option<String>,
    pub now_delay: Option<u32>,
    pub members: Vec<NodeView>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TrafficStats {
    pub download_total: u64,
    pub upload_total: u64,
    /// Bytes/sec averaged over the last poll interval.
    pub download_speed: u64,
    pub upload_speed: u64,
    pub connection_count: usize,
    /// Mihomo's reported RAM use, if exposed by this build.
    pub memory: Option<u64>,
}

/// Parsed from the subscription's pseudo-nodes (names containing `：`):
/// 到期 (expiry), 剩余 (remaining quota), 套餐 (plan), 官网 (homepage), 重置 (reset countdown).
#[derive(Debug, Clone, Default, Serialize)]
pub struct SubscriptionInfo {
    pub expiry: Option<String>,
    pub remaining: Option<String>,
    pub plan: Option<String>,
    pub homepage: Option<String>,
    pub reset: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateSnapshot {
    pub fetched_at: i64,
    pub connection: ConnectionState,
    pub groups: Vec<GroupView>,
    pub leaf_count: usize,
    pub recent_auto_switches: Vec<AutoSwitch>,
    pub traffic: TrafficStats,
    pub subscription: SubscriptionInfo,
}

impl Default for StateSnapshot {
    fn default() -> Self {
        Self {
            fetched_at: 0,
            connection: ConnectionState::Lost,
            groups: vec![],
            leaf_count: 0,
            recent_auto_switches: vec![],
            traffic: TrafficStats::default(),
            subscription: SubscriptionInfo::default(),
        }
    }
}

pub struct AppState {
    pub snapshot: RwLock<StateSnapshot>,
    pub settings: RwLock<Settings>,
    pub client: RwLock<ClashClient>,
    pub last_manual_switch: StdMutex<HashMap<String, Instant>>,
    pub recent_auto_switches: StdMutex<VecDeque<AutoSwitch>>,
    pub failure_count: AtomicU32,
    pub poll_notify: Notify,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        let client = ClashClient::new(&settings.clash_url, settings.secret.clone());
        Self {
            snapshot: RwLock::new(StateSnapshot::default()),
            settings: RwLock::new(settings),
            client: RwLock::new(client),
            last_manual_switch: StdMutex::new(HashMap::new()),
            recent_auto_switches: StdMutex::new(VecDeque::with_capacity(20)),
            failure_count: AtomicU32::new(0),
            poll_notify: Notify::new(),
        }
    }
}

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Runtime};
use tokio::time::sleep;

use crate::clash::{ConnectionsResponse, ProxiesResponse, Proxy};
use crate::country::parse_country;
use crate::events::{AutoSwitch, ConnectionEvent};
use crate::settings::Settings;
use crate::state::{
    AppState, ConnectionState, GroupKind, GroupView, NodeView, StateSnapshot, SubscriptionInfo,
    TrafficStats,
};

const GRACE: Duration = Duration::from_secs(3);
const MAX_FAILS: u32 = 3;
const SLOW_POLL_MIN_MS: u64 = 10_000;
const RING_CAP: usize = 20;

pub const GROUP_TYPES: &[&str] = &["Selector", "URLTest", "Fallback", "LoadBalance", "Relay"];

pub fn spawn<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move { run(app, state).await });
}

async fn run<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    let mut fresh_baseline = true;
    // The default snapshot starts in `Lost` so the first poll always looks like a
    // reconnect. Suppress the "reconnected" toast until we've seen at least one
    // successful poll — otherwise every app launch fires a stale notification.
    let mut had_successful_poll = false;

    loop {
        let was_lost = state.snapshot.read().await.connection == ConnectionState::Lost;
        let base_interval = state.settings.read().await.poll_interval_ms;
        let effective = if was_lost {
            base_interval.max(SLOW_POLL_MIN_MS)
        } else {
            base_interval
        };

        tokio::select! {
            _ = sleep(Duration::from_millis(effective)) => {},
            _ = state.poll_notify.notified() => {},
        }

        let fetch = {
            let client = state.client.read().await;
            client.proxies().await
        };
        // /connections is best-effort — failure shouldn't drop the whole tick.
        let connections = {
            let client = state.client.read().await;
            client.connections().await.ok()
        };

        match fetch {
            Ok(p) => {
                let settings = state.settings.read().await.clone();

                if was_lost {
                    let base = state.client.read().await.base().to_string();
                    let _ = app.emit(
                        "connection-restored",
                        ConnectionEvent { url: base.clone(), error: None },
                    );
                    if had_successful_poll && settings.notify_connection {
                        crate::notify::connection_restored(&app, &base);
                    }
                    fresh_baseline = true;
                }
                had_successful_poll = true;
                state.failure_count.store(0, Ordering::SeqCst);

                let prev = state.snapshot.read().await.clone();
                let ring_now: Vec<AutoSwitch> = state
                    .recent_auto_switches
                    .lock()
                    .unwrap()
                    .iter()
                    .cloned()
                    .collect();
                let mut next = build_snapshot(&p, &settings, ring_now);
                next.traffic = compute_traffic(connections.as_ref(), &prev, effective);
                next.subscription = parse_subscription(&p);

                let detect = !fresh_baseline && prev.connection == ConnectionState::Connected;
                if detect {
                    detect_auto_switches(&app, &state, &prev, &next, settings.notify_auto_switch).await;
                    next.recent_auto_switches = state
                        .recent_auto_switches
                        .lock()
                        .unwrap()
                        .iter()
                        .cloned()
                        .collect();
                }
                fresh_baseline = false;

                *state.snapshot.write().await = next.clone();
                let _ = app.emit("state-updated", next);
            }
            Err(e) => {
                let n = state.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                if n == MAX_FAILS {
                    let base = state.client.read().await.base().to_string();
                    {
                        let mut snap = state.snapshot.write().await;
                        snap.connection = ConnectionState::Lost;
                    }
                    let _ = app.emit(
                        "connection-lost",
                        ConnectionEvent {
                            url: base.clone(),
                            error: Some(e.to_string()),
                        },
                    );
                    if state.settings.read().await.notify_connection {
                        crate::notify::connection_lost(&app, &base);
                    }
                    let snap = state.snapshot.read().await.clone();
                    let _ = app.emit("state-updated", snap);
                }
            }
        }
    }
}

pub fn build_snapshot(
    p: &ProxiesResponse,
    settings: &Settings,
    recent: Vec<AutoSwitch>,
) -> StateSnapshot {
    let mut groups: Vec<GroupView> = p
        .proxies
        .values()
        .filter(|x| GROUP_TYPES.contains(&x.proxy_type.as_str()))
        .map(|g: &Proxy| {
            let now_country = g
                .now
                .as_ref()
                .map(|name| parse_country(name, &settings.country_overrides));
            let now_delay = g.now.as_ref().and_then(|name| {
                p.proxies
                    .get(name)
                    .and_then(|n| n.history.last().map(|h| h.delay))
            });
            let kind = group_kind(&g.proxy_type);
            let is_primary = settings.primary_group.as_deref() == Some(g.name.as_str());

            let members: Vec<NodeView> = g
                .all
                .iter()
                .filter_map(|m| {
                    p.proxies.get(m).map(|n| NodeView {
                        name: n.name.clone(),
                        country: parse_country(&n.name, &settings.country_overrides),
                        kind: n.proxy_type.clone(),
                        latest_delay: n.history.last().map(|h| h.delay),
                        is_group: GROUP_TYPES.contains(&n.proxy_type.as_str()),
                    })
                })
                .collect();

            GroupView {
                name: g.name.clone(),
                kind,
                now: g.now.clone(),
                now_country,
                now_delay,
                members,
                is_primary,
            }
        })
        .collect();

    groups.sort_by(|a, b| match (a.is_primary, b.is_primary) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    let leaf_count = p
        .proxies
        .values()
        .filter(|x| !GROUP_TYPES.contains(&x.proxy_type.as_str()))
        .count();

    StateSnapshot {
        fetched_at: now_millis(),
        connection: ConnectionState::Connected,
        groups,
        leaf_count,
        recent_auto_switches: recent,
        // Filled in by the caller (poll loop) — `build_snapshot` doesn't have access
        // to the previous snapshot or the connections-endpoint response.
        traffic: TrafficStats::default(),
        subscription: SubscriptionInfo::default(),
    }
}

fn compute_traffic(
    connections: Option<&ConnectionsResponse>,
    prev: &StateSnapshot,
    interval_ms: u64,
) -> TrafficStats {
    let Some(c) = connections else {
        // Preserve last-known totals when the connections fetch failed (e.g. transient
        // network blip). Speeds drop to 0 so the UI doesn't lie about activity.
        return TrafficStats {
            download_total: prev.traffic.download_total,
            upload_total: prev.traffic.upload_total,
            download_speed: 0,
            upload_speed: 0,
            connection_count: prev.traffic.connection_count,
            memory: prev.traffic.memory,
        };
    };

    // Mihomo restarts reset totals to 0; clamp negative deltas so speed shows 0
    // (not a wild huge number that wraps around u64).
    let dl_delta = c.download_total.saturating_sub(prev.traffic.download_total);
    let ul_delta = c.upload_total.saturating_sub(prev.traffic.upload_total);
    let secs = (interval_ms as f64 / 1000.0).max(0.001);
    let download_speed = (dl_delta as f64 / secs) as u64;
    let upload_speed = (ul_delta as f64 / secs) as u64;

    TrafficStats {
        download_total: c.download_total,
        upload_total: c.upload_total,
        download_speed,
        upload_speed,
        connection_count: c.connections.len(),
        memory: c.memory,
    }
}

/// Parse the subscription pseudo-nodes (any proxy name containing the fullwidth colon
/// `：`). Keys are in Chinese; this works for the user's yafhome subscription and
/// likely most other Chinese-provider templates that use the same convention.
fn parse_subscription(p: &ProxiesResponse) -> SubscriptionInfo {
    let mut s = SubscriptionInfo::default();
    for proxy in p.proxies.values() {
        let Some((key, value)) = proxy.name.split_once('\u{ff1a}') else {
            continue;
        };
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        match key.trim() {
            "到期" => s.expiry = Some(value),
            "剩余" => s.remaining = Some(value),
            "套餐" => s.plan = Some(value),
            "官网" => s.homepage = Some(value),
            "重置" => s.reset = Some(value),
            _ => {}
        }
    }
    s
}

fn group_kind(t: &str) -> GroupKind {
    match t {
        "URLTest" => GroupKind::UrlTest,
        "Fallback" => GroupKind::Fallback,
        "LoadBalance" => GroupKind::LoadBalance,
        "Relay" => GroupKind::Relay,
        _ => GroupKind::Selector,
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

async fn detect_auto_switches<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    prev: &StateSnapshot,
    next: &StateSnapshot,
    notify: bool,
) {
    // Auto-style groups are the canonical "switch happened" case. We also watch
    // non-GLOBAL Selector groups so that external changes (e.g. via MetaCubeXD or a
    // direct API call) toast too. Our own switch_proxy/quick_switch update the
    // snapshot optimistically, so a switch initiated from inside this app won't
    // produce a false toast — prev_now will already match next_now next tick.
    let auto_kinds = [GroupKind::UrlTest, GroupKind::Fallback, GroupKind::LoadBalance];
    let now = Instant::now();

    for g_next in &next.groups {
        let watched = auto_kinds.contains(&g_next.kind)
            || (g_next.kind == GroupKind::Selector && g_next.name != "GLOBAL");
        if !watched {
            continue;
        }
        let prev_group = prev.groups.iter().find(|p| p.name == g_next.name);
        let prev_now = prev_group.and_then(|p| p.now.as_deref());
        let next_now = g_next.now.as_deref();

        if prev_now == next_now || next_now.is_none() || prev_now.is_none() {
            continue;
        }

        let recent_manual = {
            let manual = state.last_manual_switch.lock().unwrap();
            manual
                .get(&g_next.name)
                .map(|t| now.duration_since(*t) < GRACE)
                .unwrap_or(false)
        };
        if recent_manual {
            continue;
        }

        let event = AutoSwitch {
            group: g_next.name.clone(),
            from: prev_now.unwrap_or("").to_string(),
            to: next_now.unwrap_or("").to_string(),
            from_country: prev_group
                .and_then(|p| p.now_country.clone())
                .unwrap_or_else(|| "XX".to_string()),
            to_country: g_next
                .now_country
                .clone()
                .unwrap_or_else(|| "XX".to_string()),
            at: now_millis(),
        };

        {
            let mut ring = state.recent_auto_switches.lock().unwrap();
            if ring.len() >= RING_CAP {
                ring.pop_front();
            }
            ring.push_back(event.clone());
        }

        let _ = app.emit("auto-switched", event.clone());
        if notify {
            crate::notify::auto_switch(app, &event);
        }
    }
}

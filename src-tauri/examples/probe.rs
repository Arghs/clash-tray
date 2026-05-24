//! Sanity-check probe for a live Clash/Mihomo external-controller.
//! Usage:
//!     cargo run --example probe -- http://192.168.8.1:9090 [secret]
//!
//! Output is intentionally machine-greppable so node names can be copied
//! into the country-parser test corpus (Phase 5).

use std::env;
use std::process::ExitCode;

use clash_tray_lib::clash::{ClashClient, ClashError, Proxy};

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let base = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "http://192.168.8.1:9090".to_string());
    let secret = args.get(2).cloned();

    println!("=== Clash Tray probe ===");
    println!("URL:    {base}");
    println!(
        "Secret: {}",
        secret.as_ref().map(|_| "set").unwrap_or("unset")
    );
    println!();

    let client = ClashClient::new(base.clone(), secret);

    print!("[GET /version] ");
    match client.version().await {
        Ok(v) => {
            let flags = match (v.meta, v.premium) {
                (true, true) => " (meta, premium)",
                (true, false) => " (meta)",
                (false, true) => " (premium)",
                _ => "",
            };
            println!("ok — version: {}{flags}", v.version);
        }
        Err(e) => {
            println!("FAIL");
            print_error(&e);
            return ExitCode::from(1);
        }
    }

    print!("[GET /proxies] ");
    let proxies = match client.proxies().await {
        Ok(p) => {
            println!("ok — {} entries", p.proxies.len());
            p.proxies
        }
        Err(e) => {
            println!("FAIL");
            print_error(&e);
            return ExitCode::from(1);
        }
    };

    let mut entries: Vec<&Proxy> = proxies.values().collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    let group_kinds = ["Selector", "URLTest", "Fallback", "LoadBalance", "Relay"];
    let groups: Vec<&Proxy> = entries
        .iter()
        .copied()
        .filter(|p| group_kinds.contains(&p.proxy_type.as_str()))
        .collect();
    let leaves: Vec<&Proxy> = entries
        .iter()
        .copied()
        .filter(|p| !group_kinds.contains(&p.proxy_type.as_str()))
        .collect();

    println!();
    println!("=== Groups ({}) ===", groups.len());
    for g in &groups {
        let now = g.now.as_deref().unwrap_or("—");
        println!(
            "  {:<13} {:<24}  now={:<24} members={}",
            format!("[{}]", g.proxy_type),
            g.name,
            now,
            g.all.len()
        );
    }

    println!();
    println!("=== Leaf node names ({}) ===", leaves.len());
    for n in &leaves {
        println!("  {}", n.name);
    }

    println!();
    println!("=== Leaf node detail (first 30) ===");
    for n in leaves.iter().take(30) {
        let delay = n
            .history
            .last()
            .map(|h| format!("{} ms", h.delay))
            .unwrap_or_else(|| "—".to_string());
        println!(
            "  [{:<14}] {:<32}  latest={}",
            n.proxy_type, n.name, delay
        );
    }

    ExitCode::SUCCESS
}

fn print_error(e: &ClashError) {
    match e {
        ClashError::Network(inner) => {
            eprintln!("  network error: {inner}");
            if inner.is_timeout() {
                eprintln!("  hint: connection timed out — is the router reachable from this machine?");
            } else if inner.is_connect() {
                eprintln!("  hint: connect failed — wrong host/port, or Clash external-controller not listening?");
            }
        }
        ClashError::Auth => {
            eprintln!("  authentication failed (401)");
            eprintln!("  hint: pass the external-controller-secret as the 2nd arg");
        }
        ClashError::BadStatus(s) => eprintln!("  unexpected status: {s}"),
        ClashError::Decode(d) => eprintln!("  malformed response body: {d}"),
    }
}

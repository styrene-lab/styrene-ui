use dioxus::prelude::*;
use std::collections::{BTreeMap, HashSet};

use crate::state::{PageAddress, PageView, PathEntry, PeerEntry, PeerRole};

#[derive(Clone, Debug, PartialEq)]
struct HostRow {
    hash: String,
    name: String,
    evidence: String,
    announced: String,
}

fn announce_age(timestamp: Option<i64>, now: i64) -> String {
    match timestamp {
        None => "Announcement time unknown".into(),
        Some(t) if t > now => "Announcement time uncertain".into(),
        Some(t) => {
            let seconds = now - t;
            let age = if seconds < 60 {
                "just now".into()
            } else if seconds < 3600 {
                format!("{}m ago", seconds / 60)
            } else if seconds < 86400 {
                format!("{}h ago", seconds / 3600)
            } else {
                format!("{}d ago", seconds / 86400)
            };
            format!("Announced {age}")
        }
    }
}

fn rows(
    peers: &[PeerEntry],
    paths: &[PathEntry],
    visits: &BTreeMap<String, String>,
    query: &str,
    now: i64,
) -> Vec<HostRow> {
    let routes: HashSet<_> = paths
        .iter()
        .filter(|p| p.expires.is_none_or(|expiry| expiry > now))
        .map(|p| p.destination_hash.as_str())
        .collect();
    let query = query.trim().to_lowercase();
    let mut rows: Vec<_> = peers
        .iter()
        .filter(|p| p.node_role == PeerRole::PageHost)
        .filter(|p| {
            p.hash.to_lowercase().contains(&query)
                || p.name.as_deref().unwrap_or_default().to_lowercase().contains(&query)
        })
        .map(|p| HostRow {
            hash: p.hash.clone(),
            name: p.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| p.hash.clone()),
            evidence: visits.get(&p.hash).cloned().unwrap_or_else(|| {
                if routes.contains(p.hash.as_str()) {
                    "Route listed · not tested"
                } else {
                    "No route in current table"
                }
                .into()
            }),
            announced: announce_age(p.last_announce, now),
        })
        .collect();
    rows.sort_by(|a, b| {
        a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.hash.cmp(&b.hash))
    });
    rows
}

#[component]
pub fn BrowseDirectory(
    peers: Vec<PeerEntry>,
    paths: Vec<PathEntry>,
    visits: BTreeMap<String, String>,
    page: Option<PageView>,
    local_pages: Vec<styrene_ipc::types::PageInfo>,
    available: bool,
    unavailable_reason: Option<String>,
    on_open: EventHandler<PageAddress>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut limit = use_signal(|| 20usize);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    let hosts = rows(&peers, &paths, &visits, &query.read(), now);
    rsx! {
        aside { class: "browse-directory", aria_label: "Browse directory",
            header { class: "browse-directory-heading",
                h2 { "Discover" }
                p { "Announcements identify page hosts, not live availability." }
                input { r#type: "search", aria_label: "Find a page host", placeholder: "Find by name or address…",
                    value: "{query}", oninput: move |event| { query.set(event.value()); limit.set(20); } }
                span { class: "directory-count", "{hosts.len()} matching hosts" }
            }
            if let Some(reason) = unavailable_reason { p { class: "directory-reason", "{reason}" } }
            details { class: "local-pages",
                summary { "On this node" }
                button { disabled: !available, onclick: move |_| on_open.call(PageAddress::local_index()), "Open local index" }
                for entry in local_pages.iter().filter(|p| p.kind == "page") {
                    { let path = entry.path.clone();
                        rsx! { button { disabled: !available || !entry.handler_active,
                            onclick: move |_| if let Ok(address) = PageAddress::parse(&path) { on_open.call(address); }, "{entry.path}" } }
                    }
                }
            }
            div { class: "directory-results",
                if hosts.is_empty() { p { class: "directory-reason", "No matching page hosts. Try another search or enter a page address." } }
                for host in hosts.iter().take(*limit.read()) {
                    {
                        let hash = host.hash.clone();
                        let selected = page.as_ref().is_some_and(|p| p.host == hash);
                        let pending = selected && page.as_ref().is_some_and(|p| p.loading);
                        rsx! {
                            button { key: "{hash}", class: if selected { "directory-host selected" } else { "directory-host" },
                                disabled: !available, title: "{host.name} · {host.hash}",
                                aria_label: "Open page from {host.name}",
                                onclick: move |_| if let Ok(address) = PageAddress::remote_index(&hash) { on_open.call(address); },
                                strong { "{host.name}" }
                                span { class: "directory-evidence", if pending { "Waiting for page…" } else { "{host.evidence}" } }
                                small { "{host.announced}" }
                            }
                        }
                    }
                }
                if hosts.len() > *limit.read() {
                    button { class: "directory-more", onclick: move |_| { let next = *limit.read() + 20; limit.set(next); }, "Show 20 more" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn host(name: &str, hash: &str) -> PeerEntry {
        PeerEntry {
            hash: hash.into(),
            identity_hash: None,
            name: Some(name.into()),
            status: "online".into(),
            node_role: PeerRole::PageHost,
            capabilities: vec![],
            version: None,
            last_announce: Some(60),
            announce_count: 1,
        }
    }
    #[test]
    fn online_announcement_is_not_available_and_search_is_case_insensitive() {
        let hosts = rows(&[host("Zoo", "b"), host("Alpha", "a")], &[], &BTreeMap::new(), "", 360);
        assert_eq!(hosts[0].name, "Alpha");
        assert_eq!(hosts[0].evidence, "No route in current table");
        assert_eq!(hosts[0].announced, "Announced 5m ago");
        assert_eq!(rows(&[host("Alpha", "a")], &[], &BTreeMap::new(), "ALP", 360).len(), 1);
    }
    #[test]
    fn last_attempt_overrides_inferred_availability() {
        let visits = BTreeMap::from([("a".into(), "Last attempt failed".into())]);
        assert_eq!(
            rows(&[host("Alpha", "a")], &[], &visits, "", 360)[0].evidence,
            "Last attempt failed"
        );
        assert_eq!(announce_age(Some(400), 360), "Announcement time uncertain");
    }
}

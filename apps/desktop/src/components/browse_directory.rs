use dioxus::prelude::*;
use std::collections::{BTreeMap, HashSet};

use crate::state::{PageAddress, PageView, PathEntry, PeerEntry, PeerRole};

#[derive(Clone, Debug, PartialEq)]
struct HostRow {
    hash: String,
    name: String,
    evidence: String,
    announced: String,
    timestamp: Option<i64>,
}

fn announce_age(timestamp: Option<i64>, now: i64) -> String {
    match timestamp {
        None => "Announcement time unknown".into(),
        Some(t) if t > now => "Announcement time uncertain".into(),
        Some(t) => {
            let seconds = now.saturating_sub(t);
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
            timestamp: p.last_announce,
        })
        .collect();
    rows.sort_by(|a, b| {
        a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.hash.cmp(&b.hash))
    });
    rows
}

type Favorites = BTreeMap<String, String>;

fn favorites_path() -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
        })
        .map(|root| root.join("styrene-ui/browse-favorites.json"))
}

fn read_favorites(path: &std::path::Path) -> Result<Favorites, String> {
    use std::io::Read;
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Favorites::new()),
        Err(_) => return Err("Could not read saved favorites.".into()),
    };
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes).map_err(|_| "Could not read saved favorites.")?;
    if bytes.len() > 65536 {
        return Err("Saved favorites exceed the storage limit.".into());
    }
    let favorites: Favorites =
        serde_json::from_slice(&bytes).map_err(|_| "Saved favorites are invalid.")?;
    if favorites.len() > 256
        || favorites.iter().any(|(hash, name)| {
            hash.len() != 32 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) || name.len() > 128
        })
    {
        return Err("Saved favorites are invalid.".into());
    }
    Ok(favorites)
}

fn write_favorites(path: &std::path::Path, favorites: &Favorites) -> Result<(), String> {
    if favorites.len() > 256 {
        return Err("You can save up to 256 favorites.".into());
    }
    let bytes = serde_json::to_vec(favorites).map_err(|_| "Could not encode favorites.")?;
    if bytes.len() > 65536 {
        return Err("Favorites exceed the storage limit.".into());
    }
    let parent = path.parent().ok_or("Favorites storage is unavailable.")?;
    std::fs::create_dir_all(parent).map_err(|_| "Could not create favorites storage.")?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes)
        .and_then(|_| std::fs::rename(&temporary, path))
        .map_err(|_| "Could not save favorites.".into())
}

struct DirectoryFilter<'a> {
    query: &'a str,
    collection: &'a str,
    age: i64,
    newest: bool,
    now: i64,
}

fn filter_rows(
    mut hosts: Vec<HostRow>,
    favorites: &Favorites,
    visits: &BTreeMap<String, String>,
    filter: DirectoryFilter<'_>,
) -> Vec<HostRow> {
    let DirectoryFilter { query, collection, age, newest, now } = filter;
    let query = query.trim().to_lowercase();
    for (hash, name) in favorites {
        if !hosts.iter().any(|row| &row.hash == hash)
            && (hash.to_lowercase().contains(&query) || name.to_lowercase().contains(&query))
        {
            hosts.push(HostRow {
                hash: hash.clone(),
                name: name.clone(),
                evidence: visits
                    .get(hash)
                    .cloned()
                    .unwrap_or_else(|| "Saved host · not currently announced".into()),
                announced: announce_age(None, now),
                timestamp: None,
            });
        }
    }
    hosts.retain(|row| {
        (collection != "favorites" || favorites.contains_key(&row.hash))
            && (collection != "attempted" || visits.contains_key(&row.hash))
            && (age == 0 || row.timestamp.is_some_and(|t| t <= now && now.saturating_sub(t) <= age))
    });
    hosts.sort_by(|a, b| {
        let time = |row: &HostRow| row.timestamp.filter(|t| *t <= now);
        (if newest { time(b).cmp(&time(a)) } else { std::cmp::Ordering::Equal })
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then(a.hash.cmp(&b.hash))
    });
    hosts
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
    let mut collection = use_signal(|| "all".to_string());
    let mut age = use_signal(|| 0i64);
    let mut newest = use_signal(|| true);
    let loaded = use_hook(|| {
        favorites_path()
            .ok_or_else(|| "Favorites storage is unavailable.".to_string())
            .and_then(|path| read_favorites(&path))
    });
    let storage_unreadable = loaded.is_err();
    let initial_error = loaded.as_ref().err().cloned();
    let mut favorites = use_signal(|| loaded.unwrap_or_default());
    let mut storage_error = use_signal(|| initial_error);
    let mut tick = use_signal(|| 0u64);
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let next = *tick.read() + 1;
            tick.set(next);
        }
    });
    let _ = tick();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    let hosts = filter_rows(
        rows(&peers, &paths, &visits, &query.read(), now),
        &favorites.read(),
        &visits,
        DirectoryFilter {
            query: &query.read(),
            collection: &collection.read(),
            age: age(),
            newest: newest(),
            now,
        },
    );
    rsx! {
        aside { class: "browse-directory", aria_label: "Browse directory",
            header { class: "browse-directory-heading",
                h2 { "Directory" }
                p { "Announced hosts · availability unverified" }
                input { r#type: "search", aria_label: "Find a page host", placeholder: "Find by name or address…",
                    value: "{query}", oninput: move |event| { query.set(event.value()); limit.set(20); } }
                div { class: "directory-collections", role: "group", aria_label: "Host collection",
                    for (value, label) in [("all", "All hosts"), ("favorites", "Favorites"), ("attempted", "Attempted")] {
                        button { r#type: "button", aria_pressed: collection() == value, onclick: move |_| { collection.set(value.into()); limit.set(20); }, "{label}" }
                    }
                }
                details { class: "directory-filters",
                    summary { "Filters & sorting" }
                    div { class: "directory-sources", role: "group", aria_label: "Browse sources",
                        span { class: "directory-source-active", "NomadNet" }
                        button { disabled: true, title: "I2P browsing is planned", "I2P · later" }
                        button { disabled: true, title: "Yggdrasil browsing is planned", "Yggdrasil · later" }
                    }
                    div { class: "directory-filter-grid",
                    label { "Last announced"
                        select { value: "{age}", onchange: move |event| { age.set(event.value().parse().unwrap_or(0)); limit.set(20); },
                            option { value: "0", "Any time" } option { value: "3600", "Past hour" } option { value: "86400", "Past day" } option { value: "604800", "Past week" }
                        }
                    }
                    label { "Sort by"
                        select { value: if newest() { "newest" } else { "name" }, onchange: move |event| { newest.set(event.value() == "newest"); limit.set(20); },
                            option { value: "newest", "Newest first" } option { value: "name", "Name A–Z" }
                        }
                    }
                    }
                    label { class: "directory-cache", title: "The provider does not expose a cache inventory. Past loads do not guarantee offline access.",
                        input { r#type: "checkbox", disabled: true } "Cached" span { "Inventory unavailable" }
                    }
                }
                div { class: "directory-result-summary",
                    span { class: "directory-count", "Showing {hosts.len().min(limit())} of {hosts.len()} matches" }
                    if !query().is_empty() || collection() != "all" || age() != 0 || !newest() {
                        button { onclick: move |_| { query.set(String::new()); collection.set("all".into()); age.set(0); newest.set(true); limit.set(20); }, "Reset" }
                    }
                }
                if age() != 0 { small { "Announced within {age() / 3600}h" } }
                if collection() == "attempted" { small { "Attempts in this backend session" } }
                if let Some(error) = storage_error() { p { role: "alert", "{error}" } }
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
                if hosts.is_empty() { p { class: "directory-reason",
                    if collection() == "favorites" && favorites.read().is_empty() { "No favorites yet. Star a host in All hosts to keep it here." }
                    else if collection() == "attempted" && visits.is_empty() { "No attempts in this session yet. Open a host from All hosts." }
                    else { "No matching page hosts. Reset filters, try another search or enter a page address." }
                } }
                for host in hosts.iter().take(*limit.read()) {
                    {
                        let hash = host.hash.clone();
                        let selected = page.as_ref().is_some_and(|p| p.host == hash);
                        let favorite_hash = hash.clone();
                        let favorite_name: String = host.name.chars().scan(0usize, |length, c| { *length += c.len_utf8(); (*length <= 128).then_some(c) }).collect();
                        let saved = favorites.read().contains_key(&hash);
                        let pending = selected && page.as_ref().is_some_and(|p| p.loading);
                        rsx! {
                            div { key: "{hash}", class: "directory-row",
                            button { class: if selected { "directory-host selected" } else { "directory-host" },
                                disabled: !available, title: "{host.name} · {host.hash}",
                                aria_label: "Open page from {host.name}",
                                onclick: move |_| if let Ok(address) = PageAddress::remote_index(&hash) { on_open.call(address); },
                                strong { "{host.name}" }
                                span { class: "directory-evidence", if pending { "Waiting for page…" } else { "{host.evidence}" } }
                                small { "{host.announced}" }
                            }
                            button { class: "directory-favorite", disabled: storage_unreadable, aria_label: if saved { format!("Remove {} from favorites", host.name) } else { format!("Favorite {}", host.name) }, aria_pressed: saved,
                                onclick: move |_| {
                                    let mut next = favorites();
                                    if next.remove(&favorite_hash).is_none() { next.insert(favorite_hash.clone(), favorite_name.clone()); }
                                    let result = favorites_path().ok_or_else(|| "Favorites storage is unavailable.".to_string()).and_then(|path| write_favorites(&path, &next));
                                    match result { Ok(()) => { favorites.set(next); storage_error.set(None); }, Err(error) => storage_error.set(Some(error)) }
                                },
                                if saved { "★" } else { "☆" }
                            }
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
    #[test]
    fn filters_compose_and_unknown_or_future_times_do_not_look_recent() {
        let mut future = host("Future", "f");
        future.last_announce = Some(1001);
        let mut unknown = host("Unknown", "u");
        unknown.last_announce = None;
        let favorites =
            BTreeMap::from([("a".into(), "Alpha".into()), ("missing".into(), "Saved".into())]);
        let visits = BTreeMap::from([("a".into(), "Last attempt failed".into())]);
        let base = rows(&[host("Alpha", "a"), future, unknown], &[], &visits, "", 1000);
        let recent = filter_rows(
            base.clone(),
            &favorites,
            &visits,
            DirectoryFilter { query: "", collection: "all", age: 3600, newest: true, now: 1000 },
        );
        assert_eq!(recent.iter().map(|r| r.hash.as_str()).collect::<Vec<_>>(), vec!["a"]);
        let saved = filter_rows(
            base.clone(),
            &favorites,
            &visits,
            DirectoryFilter {
                query: "",
                collection: "favorites",
                age: 0,
                newest: false,
                now: 1000,
            },
        );
        assert_eq!(saved.len(), 2);
        assert_eq!(saved[1].evidence, "Saved host · not currently announced");
        assert_eq!(
            filter_rows(
                base,
                &favorites,
                &visits,
                DirectoryFilter {
                    query: "",
                    collection: "attempted",
                    age: 0,
                    newest: false,
                    now: 1000
                }
            )
            .len(),
            1
        );
        assert!(
            filter_rows(
                vec![],
                &favorites,
                &visits,
                DirectoryFilter {
                    query: "NO MATCH",
                    collection: "favorites",
                    age: 0,
                    newest: false,
                    now: 1000
                }
            )
            .is_empty()
        );
    }

    #[test]
    fn favorites_round_trip_and_reject_corrupt_or_oversized_storage() {
        let root = std::env::temp_dir().join(format!(
            "styrene-favorites-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let path = root.join("favorites.json");
        assert_eq!(read_favorites(&path).expect("absent favorites"), Favorites::new());
        let favorites = BTreeMap::from([("ab".repeat(16), "Pi Browse Lab".into())]);
        write_favorites(&path, &favorites).expect("save favorites");
        assert_eq!(read_favorites(&path).expect("read favorites"), favorites);
        std::fs::write(&path, b"invalid").expect("write invalid fixture");
        assert!(read_favorites(&path).is_err());
        std::fs::write(&path, vec![b' '; 65537]).expect("write oversized fixture");
        assert!(read_favorites(&path).is_err());
        std::fs::remove_dir_all(root).expect("remove fixture directory");
    }
    #[test]
    fn newest_sort_orders_known_times_before_unknown_and_breaks_ties_by_name() {
        let mut recent = host("Zulu", "z");
        recent.last_announce = Some(900);
        let mut tie = host("Beta", "b");
        tie.last_announce = Some(900);
        let mut unknown = host("Aardvark", "u");
        unknown.last_announce = None;
        let ordered = filter_rows(
            rows(&[host("Alpha", "a"), recent, unknown, tie], &[], &BTreeMap::new(), "", 1000),
            &Favorites::new(),
            &BTreeMap::new(),
            DirectoryFilter { query: "", collection: "all", age: 0, newest: true, now: 1000 },
        );
        assert_eq!(
            ordered.iter().map(|r| r.hash.as_str()).collect::<Vec<_>>(),
            vec!["b", "z", "a", "u"]
        );
    }
}

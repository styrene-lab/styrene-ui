//! Join transport next-hop observations to discovered identities for display.
//! This is routing evidence, not authentication of a configured DNS name.
use std::collections::BTreeMap;

use crate::state::{PathEntry, PeerEntry};
use styrene_ipc::types::ManagedInterface;

#[derive(Debug, PartialEq)]
pub(crate) struct ObservedIdentity {
    pub hash: String,
    pub route_count: usize,
    pub destinations: Vec<PeerEntry>,
}

pub(crate) fn connection_identities(
    connection: &ManagedInterface,
    paths: &[PathEntry],
    peers: &[PeerEntry],
    now: i64,
) -> Vec<ObservedIdentity> {
    let Some(interface) = connection.runtime_hash.as_deref() else { return Vec::new() };
    if !matches!(connection.state.as_str(), "connected" | "active" | "online") {
        return Vec::new();
    }
    let mut identities = BTreeMap::<String, usize>::new();
    for path in paths.iter().filter(|path| {
        path.interface == interface
            && !path.observation.stale
            && path.expires.is_none_or(|expiry| expiry > now)
            && path.hops > 1
            // Direct announces use their destination as received_from. That is
            // an application address, not a forwarding transport identity.
            && path.next_hop != path.destination_hash
            && path.next_hop.len() == 32
            && path.next_hop.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        *identities.entry(path.next_hop.to_ascii_lowercase()).or_default() += 1;
    }
    identities
        .into_iter()
        .map(|(hash, route_count)| {
            let mut destinations: Vec<_> = peers
                .iter()
                .filter(|peer| {
                    peer.identity_hash
                        .as_deref()
                        .is_some_and(|identity| identity.eq_ignore_ascii_case(&hash))
                })
                .cloned()
                .collect();
            destinations.sort_by(|a, b| a.hash.cmp(&b.hash));
            destinations.dedup_by(|a, b| a.hash == b.hash);
            ObservedIdentity { hash, route_count, destinations }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PeerRole;

    const ID: &str = "f18f91868ef1d3653b54500b68bf8135";
    fn connection() -> ManagedInterface {
        let mut connection = ManagedInterface::default();
        connection.runtime_hash = Some("tcp-a".into());
        connection.state = "connected".into();
        connection
    }
    fn path() -> PathEntry {
        PathEntry {
            destination_hash: "3e7ca68201940826984df92ab3ce961c".into(),
            hops: 4,
            next_hop: ID.into(),
            interface: "tcp-a".into(),
            expires: Some(200),
            observation: Default::default(),
        }
    }
    fn peer() -> PeerEntry {
        PeerEntry {
            hash: "780e7aa7b2f175c88f28c7ba8ab1b714".into(),
            identity_hash: Some(ID.into()),
            name: Some("Community Hub".into()),
            status: "online".into(),
            node_role: PeerRole::Hub,
            capabilities: vec![],
            version: None,
            last_announce: None,
            announce_count: 1,
        }
    }
    #[test]
    fn joins_forwarding_identity_to_announced_destinations_without_name_matching() {
        let mut renamed = peer();
        renamed.name = Some("An unrelated display name".into());
        let found = connection_identities(&connection(), &[path()], &[renamed.clone()], 100);
        assert_eq!(
            found,
            vec![ObservedIdentity { hash: ID.into(), route_count: 1, destinations: vec![renamed] }]
        );
    }
    #[test]
    fn excludes_other_connections_stale_expired_and_direct_destination_routes() {
        let mut other = path();
        other.interface = "tcp-b".into();
        let mut stale = path();
        stale.observation.stale = true;
        let mut expired = path();
        expired.expires = Some(100);
        let mut direct = path();
        direct.next_hop = direct.destination_hash.clone();
        direct.hops = 1;
        assert!(
            connection_identities(&connection(), &[other, stale, expired, direct], &[peer()], 100)
                .is_empty()
        );
        let mut disconnected = connection();
        disconnected.state = "disconnected".into();
        assert!(connection_identities(&disconnected, &[path()], &[peer()], 100).is_empty());
    }
    #[test]
    fn retains_distinct_transport_identities_and_does_not_guess_a_peer_by_name() {
        let mut second = path();
        second.next_hop = "11111111111111111111111111111111".into();
        let mut same_name = peer();
        same_name.identity_hash = None;
        let found = connection_identities(&connection(), &[path(), second], &[same_name], 100);
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|identity| identity.destinations.is_empty()));
    }
}

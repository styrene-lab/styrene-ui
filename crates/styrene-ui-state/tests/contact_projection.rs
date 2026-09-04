//! Unit coverage for the contact projection: identity grouping, role
//! derivation, and the operator-built Contacts and Pages lists.

use styrene_ui_state::{
    ContactBook, ContactRole, Conversation, LinkMode, MobileFixture, MobileMinimumCorpus, Peer,
    PeerSource, contact_lists, project_contacts,
};

const FIXTURES: &str = include_str!("../../../tests/fixtures/mobile-minimum-v1/states.json");

fn peer(destination_hash: &str, aspect: &str, identity_hash: &str) -> Peer {
    Peer {
        destination_hash: destination_hash.into(),
        aspect: aspect.into(),
        display_name: None,
        observed_at: 1_000,
        age_secs: 1,
        source: PeerSource::CanonicalAnnounce,
        announce_count: 1,
        identity_hash: identity_hash.into(),
        hops: None,
        interface_kind: None,
    }
}

#[test]
fn identity_grouping_merges_three_aspects_into_one_contact_with_three_roles() {
    let peers = vec![
        peer("dest-delivery", "lxmf.delivery", "identity-1"),
        peer("dest-node", "nomadnetwork.node", "identity-1"),
        peer("dest-propagation", "lxmf.propagation", "identity-1"),
    ];
    let book = ContactBook::default();

    let contacts = project_contacts(&peers, &[], &book);

    assert_eq!(contacts.len(), 1);
    let contact = &contacts[0];
    assert_eq!(contact.id, "identity-1");
    assert_eq!(contact.roles, vec![ContactRole::Person, ContactRole::PageHost, ContactRole::Relay]);
    assert_eq!(contact.delivery_destination.as_deref(), Some("dest-delivery"));
    assert_eq!(contact.destinations.len(), 3);
}

#[test]
fn nomadnet_only_node_has_no_person_role_and_no_delivery_destination() {
    let peers = vec![peer("dest-node", "nomadnetwork.node", "identity-2")];
    let book = ContactBook::default();

    let contacts = project_contacts(&peers, &[], &book);

    assert_eq!(contacts.len(), 1);
    let contact = &contacts[0];
    assert_eq!(contact.roles, vec![ContactRole::PageHost]);
    assert!(!contact.roles.contains(&ContactRole::Person));
    assert_eq!(contact.delivery_destination, None);
}

#[test]
fn alias_wins_over_announced_name() {
    let mut announced = peer("dest-delivery", "lxmf.delivery", "identity-3");
    announced.display_name = Some("Announced Name".into());
    let mut book = ContactBook::default();
    book.aliases.insert("identity-3".into(), "Operator Alias".into());

    let contacts = project_contacts(&[announced], &[], &book);

    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].name, "Operator Alias");
    assert_eq!(contacts[0].alias.as_deref(), Some("Operator Alias"));
}

#[test]
fn favourite_with_no_conversation_lands_in_contacts() {
    let peers = vec![peer("dest-delivery", "lxmf.delivery", "identity-4")];
    let mut book = ContactBook::default();
    book.favourites.insert("identity-4".into());

    let contacts = project_contacts(&peers, &[], &book);
    assert!(!contacts[0].has_conversation);
    assert!(contacts[0].favourite);

    let lists = contact_lists(&contacts);
    assert_eq!(lists.contacts.len(), 1);
    assert_eq!(lists.contacts[0].id, "identity-4");
}

#[test]
fn bookmarked_page_host_lands_in_pages_and_not_in_contacts() {
    let peers = vec![peer("dest-node", "nomadnetwork.node", "identity-5")];
    let mut book = ContactBook::default();
    book.bookmarks.insert("identity-5".into());

    let contacts = project_contacts(&peers, &[], &book);
    let lists = contact_lists(&contacts);

    assert_eq!(lists.pages.len(), 1);
    assert_eq!(lists.pages[0].id, "identity-5");
    assert!(lists.contacts.is_empty());
    assert_eq!(lists.directory.len(), 1);
}

#[test]
fn messaged_person_without_favourite_still_lands_in_contacts() {
    let peers = vec![peer("dest-delivery", "lxmf.delivery", "identity-6")];
    let conversations = vec![Conversation {
        peer_hash: "dest-delivery".into(),
        unread_count: 2,
        draft: String::new(),
        draft_revision: 0,
    }];
    let book = ContactBook::default();

    let contacts = project_contacts(&peers, &conversations, &book);
    assert!(contacts[0].has_conversation);
    assert_eq!(contacts[0].unread_count, 2);

    let lists = contact_lists(&contacts);
    assert_eq!(lists.contacts.len(), 1);
}

#[test]
fn hops_known_yields_rns_direct_link_mode() {
    let mut announced = peer("dest-delivery", "lxmf.delivery", "identity-7");
    announced.hops = Some(3);
    let contacts = project_contacts(&[announced], &[], &ContactBook::default());
    assert_eq!(contacts[0].link, LinkMode::RnsDirect);
    assert_eq!(contacts[0].hops, Some(3));
}

#[test]
fn no_hops_yields_unreachable_link_mode() {
    let announced = peer("dest-delivery", "lxmf.delivery", "identity-8");
    let contacts = project_contacts(&[announced], &[], &ContactBook::default());
    assert_eq!(contacts[0].link, LinkMode::Unreachable);
}

#[test]
fn legacy_fixture_json_still_deserializes() {
    let corpus: MobileMinimumCorpus =
        serde_json::from_str(FIXTURES).expect("legacy mobile minimum fixture must deserialize");
    assert!(!corpus.fixtures.is_empty());
    for fixture in &corpus.fixtures {
        assert_eq!(fixture.contact_book, ContactBook::default());
        for peer in &fixture.peers {
            assert_eq!(peer.identity_hash, "");
            assert_eq!(peer.hops, None);
            assert_eq!(peer.interface_kind, None);
        }
    }

    // Round-tripping through project_contacts must not panic on real data.
    for fixture in corpus.fixtures {
        let MobileFixture { peers, conversations, contact_book, .. } = fixture;
        let _ = project_contacts(&peers, &conversations, &contact_book);
    }
}

#[test]
fn conversation_with_unannounced_peer_still_lands_in_contacts() {
    let peers = vec![peer("dest-other", "lxmf.delivery", "identity-8")];
    let conversations = vec![Conversation {
        peer_hash: "f396e895aaaaaaaaaaaaaaaaaaaaaaaa".into(),
        unread_count: 1,
        draft: String::new(),
        draft_revision: 0,
    }];
    let book = ContactBook::default();

    let contacts = project_contacts(&peers, &conversations, &book);
    let quiet = contacts
        .iter()
        .find(|contact| contact.id == "f396e895aaaaaaaaaaaaaaaaaaaaaaaa")
        .expect("conversation-only peer is projected");
    assert!(quiet.has_conversation);
    assert_eq!(quiet.unread_count, 1);
    assert_eq!(quiet.name, "Peer f396e895");
    assert_eq!(quiet.roles, vec![ContactRole::Person]);
    assert_eq!(quiet.link, LinkMode::Unreachable);
    assert_eq!(quiet.announce_count, 0);

    let lists = contact_lists(&contacts);
    assert_eq!(lists.contacts.len(), 1);
    assert_eq!(lists.contacts[0].id, quiet.id);
}

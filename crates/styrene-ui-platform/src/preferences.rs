//! Generic string-keyed preference persistence, and the contact book
//! helpers built on it.
//!
//! Apple adapters back this with `NSUserDefaults`. Platforms without a
//! durable backend yet use [`MemoryPreferenceStore`], which persists only
//! for the lifetime of the process.

use std::collections::HashMap;
use std::sync::Mutex;

use styrene_ui_state::ContactBook;

/// A durable string-keyed preference store.
///
/// Implementations must be safe to share across the async session loop, so
/// reads and writes take `&self` rather than `&mut self`.
pub trait PreferenceStore: Send + Sync {
    fn load(&self, key: &str) -> Option<String>;
    fn store(&self, key: &str, value: &str);
}

/// Deterministic in-memory preference store for tests and for platforms that
/// have no durable backend implementation yet.
#[derive(Debug, Default)]
pub struct MemoryPreferenceStore {
    values: Mutex<HashMap<String, String>>,
}

impl PreferenceStore for MemoryPreferenceStore {
    fn load(&self, key: &str) -> Option<String> {
        self.values.lock().ok()?.get(key).cloned()
    }

    fn store(&self, key: &str, value: &str) {
        if let Ok(mut values) = self.values.lock() {
            values.insert(key.to_owned(), value.to_owned());
        }
    }
}

/// Preference key for the operator's contact book.
pub const CONTACT_BOOK_KEY: &str = "io.styrene.contact-book";

/// Load the contact book from `store`.
///
/// An absent or unparsable value falls back to an empty book, so a corrupt
/// store fails toward losing operator preferences rather than toward a panic
/// or a stale book.
#[must_use]
pub fn load_contact_book(store: &dyn PreferenceStore) -> ContactBook {
    store
        .load(CONTACT_BOOK_KEY)
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}

/// Persist the contact book to `store`.
pub fn save_contact_book(store: &dyn PreferenceStore, book: &ContactBook) {
    if let Ok(encoded) = serde_json::to_string(book) {
        store.store(CONTACT_BOOK_KEY, &encoded);
    }
}

#[cfg(test)]
mod tests {
    use styrene_ui_state::DeliveryPreference;

    use super::{
        ContactBook, MemoryPreferenceStore, PreferenceStore, load_contact_book, save_contact_book,
    };

    #[test]
    fn absent_key_loads_as_an_empty_book() {
        let store = MemoryPreferenceStore::default();
        assert_eq!(load_contact_book(&store), ContactBook::default());
    }

    #[test]
    fn save_then_load_round_trips_every_field() {
        let store = MemoryPreferenceStore::default();
        let mut book = ContactBook::default();
        book.favourites.insert("contact-1".into());
        book.bookmarks.insert("contact-2".into());
        book.aliases.insert("contact-1".into(), "Alias".into());
        book.delivery_preferences.insert("contact-1".into(), DeliveryPreference::AlwaysViaNode);

        save_contact_book(&store, &book);

        assert_eq!(load_contact_book(&store), book);
    }

    #[test]
    fn malformed_stored_value_falls_back_to_an_empty_book() {
        let store = MemoryPreferenceStore::default();
        store.store(super::CONTACT_BOOK_KEY, "not json");
        assert_eq!(load_contact_book(&store), ContactBook::default());
    }
}

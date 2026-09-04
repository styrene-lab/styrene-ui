//! `NSUserDefaults` adapter for the generic preference store contract in
//! `styrene-ui-platform`.
//!
//! Only string values cross this boundary. No Objective-C object or pointer
//! escapes this crate's safe public API.

use objc2_foundation::{NSString, NSUserDefaults};
use styrene_ui_platform::PreferenceStore;

/// Bounded `NSUserDefaults` adapter for string-keyed operator preferences,
/// such as the contact book.
#[derive(Clone, Copy, Debug, Default)]
pub struct UserDefaultsPreferenceStore;

impl PreferenceStore for UserDefaultsPreferenceStore {
    fn load(&self, key: &str) -> Option<String> {
        let key = NSString::from_str(key);
        NSUserDefaults::standardUserDefaults().stringForKey(&key).map(|value| value.to_string())
    }

    fn store(&self, key: &str, value: &str) {
        let defaults = NSUserDefaults::standardUserDefaults();
        let key = NSString::from_str(key);
        let value = NSString::from_str(value);
        // SAFETY: NSString is a supported NSUserDefaults property-list value.
        unsafe { defaults.setObject_forKey(Some(&value), &key) };
    }
}

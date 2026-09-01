use styrene_ui_state::{
    IdentityBackupProtection, IdentityRecoveryFailure, MAX_IDENTITY_BACKUP_PROTECTION_BYTES,
};

#[test]
fn backup_protection_is_bounded_and_debug_redacted() {
    let marker = "recovery protection marker";
    let protection = IdentityBackupProtection::new(marker.into()).unwrap();

    let debug = format!("{protection:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains(marker));
    assert_eq!(protection.into_bytes(), marker.as_bytes());
}

#[test]
fn backup_protection_rejects_empty_and_oversized_input() {
    assert_eq!(
        IdentityBackupProtection::new(String::new()).unwrap_err(),
        IdentityRecoveryFailure::ProtectionRequired
    );
    assert_eq!(
        IdentityBackupProtection::new("x".repeat(MAX_IDENTITY_BACKUP_PROTECTION_BYTES + 1))
            .unwrap_err(),
        IdentityRecoveryFailure::ProtectionTooLarge
    );
    assert!(
        IdentityBackupProtection::new("x".repeat(MAX_IDENTITY_BACKUP_PROTECTION_BYTES)).is_ok()
    );
}

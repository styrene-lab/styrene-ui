fn main() {
    dioxus::LaunchBuilder::mobile()
        .with_cfg(
            dioxus::mobile::Config::new()
                .with_custom_index(styrene_mobile::MOBILE_INDEX.to_string()),
        )
        .launch(styrene_mobile::App);
}

#[cfg(test)]
mod tests {
    const DIOXUS_CONFIG: &str = include_str!("../Dioxus.toml");

    #[test]
    fn packaging_declares_ios_compatibility_contract() {
        assert!(DIOXUS_CONFIG.contains("deployment_target = \"17.0\""));
        assert!(DIOXUS_CONFIG.contains("identifier = \"io.styrene.mesh\""));
        assert!(DIOXUS_CONFIG.contains("NSFaceIDUsageDescription"));
        assert!(DIOXUS_CONFIG.contains("[permissions]"));
        assert!(DIOXUS_CONFIG.contains("camera = { description"));
        assert!(DIOXUS_CONFIG.contains("bluetooth = { description"));
    }
}

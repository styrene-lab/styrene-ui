use dioxus::prelude::*;

const BACK_LISTENER: &str = r#"
if (!history.state?.styrenePane) {
    history.replaceState({ styrenePane: "root" }, "", location.href);
}
window.addEventListener("popstate", () => {
    document.getElementById("mobile.platform-back")?.click();
});
"#;

pub fn use_back_navigation() {
    use_effect(move || {
        document::eval(BACK_LISTENER);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listener_uses_history_state_and_fixed_dioxus_action() {
        assert!(BACK_LISTENER.contains("history.replaceState"));
        assert!(BACK_LISTENER.contains("popstate"));
        assert!(BACK_LISTENER.contains("mobile.platform-back"));
        assert!(!BACK_LISTENER.contains("window.ipc"));
    }
}

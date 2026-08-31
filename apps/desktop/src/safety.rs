use crate::backend::RuntimeProfile;
use styrene_ipc::types::{ACTIVE_CAPABILITIES_VERSION, ActiveCapabilitiesInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlPlane {
    Operate,
    Lab,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyAction {
    plane: ControlPlane,
    required_capability: Option<String>,
    requires_daemon_session: bool,
    external_network: bool,
    destructive: bool,
}

impl SafetyAction {
    pub fn operate(capability: impl Into<String>, destructive: bool) -> Self {
        Self {
            plane: ControlPlane::Operate,
            required_capability: Some(capability.into()),
            requires_daemon_session: true,
            external_network: false,
            destructive,
        }
    }

    pub fn operate_session(destructive: bool) -> Self {
        Self {
            plane: ControlPlane::Operate,
            required_capability: None,
            requires_daemon_session: true,
            external_network: false,
            destructive,
        }
    }

    pub fn fixture_scenario() -> Self {
        Self {
            plane: ControlPlane::Lab,
            required_capability: Some("fixture.scenarios".into()),
            requires_daemon_session: true,
            external_network: false,
            destructive: false,
        }
    }

    pub fn live_scenario() -> Self {
        Self {
            plane: ControlPlane::Lab,
            required_capability: None,
            requires_daemon_session: false,
            external_network: true,
            destructive: true,
        }
    }

    pub fn lab_evidence_export() -> Self {
        Self {
            plane: ControlPlane::Lab,
            required_capability: None,
            requires_daemon_session: false,
            external_network: false,
            destructive: false,
        }
    }

    pub fn is_destructive(&self) -> bool {
        self.destructive
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafetyDenialKind {
    WrongControlPlane,
    InvalidRuntime,
    ExternalNetworkBlocked,
    Disconnected,
    GenerationUnknown,
    CapabilitiesUnknown,
    CapabilitiesStale,
    Degraded,
    Unauthorized,
    GenerationChanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyDenial {
    pub kind: SafetyDenialKind,
    capability: Option<String>,
    detail: Option<String>,
}

impl SafetyDenial {
    fn new(kind: SafetyDenialKind, capability: Option<&str>, detail: Option<String>) -> Self {
        Self { kind, capability: capability.map(str::to_string), detail }
    }

    pub const fn operator_message(&self) -> &'static str {
        match self.kind {
            SafetyDenialKind::WrongControlPlane => {
                "This action is unavailable from this control surface."
            }
            SafetyDenialKind::InvalidRuntime => "No valid runtime profile is active.",
            SafetyDenialKind::ExternalNetworkBlocked => {
                "Fixture sessions cannot open external network interfaces."
            }
            SafetyDenialKind::Disconnected => "Connect to the daemon to use this action.",
            SafetyDenialKind::GenerationUnknown | SafetyDenialKind::CapabilitiesUnknown => {
                "Waiting for permissions from the active session."
            }
            SafetyDenialKind::CapabilitiesStale => {
                "Refresh session permissions before using this action."
            }
            SafetyDenialKind::Degraded => {
                "This action is temporarily unavailable in the active session."
            }
            SafetyDenialKind::Unauthorized => "The active session does not permit this action.",
            SafetyDenialKind::GenerationChanged => {
                "The session changed. Review the action before confirming it."
            }
        }
    }

    pub fn diagnostic_message(&self) -> String {
        let subject = self.capability.as_deref().unwrap_or("action");
        match self.kind {
            SafetyDenialKind::WrongControlPlane => {
                format!("{subject} is unavailable from the requested control plane")
            }
            SafetyDenialKind::InvalidRuntime => "runtime profile is invalid".into(),
            SafetyDenialKind::ExternalNetworkBlocked => {
                "Fixture profile cannot open external network interfaces".into()
            }
            SafetyDenialKind::Disconnected => "daemon is disconnected".into(),
            SafetyDenialKind::GenerationUnknown => "daemon generation is unknown".into(),
            SafetyDenialKind::CapabilitiesUnknown => "capabilities are unknown".into(),
            SafetyDenialKind::CapabilitiesStale => {
                format!("capabilities are stale ({})", self.detail.as_deref().unwrap_or("unknown"))
            }
            SafetyDenialKind::Degraded => format!(
                "{subject} unavailable: {}",
                self.detail.as_deref().unwrap_or("daemon reported degraded")
            ),
            SafetyDenialKind::Unauthorized => {
                format!("current daemon session does not authorize {subject}")
            }
            SafetyDenialKind::GenerationChanged => {
                "confirmation expired because the runtime generation changed".into()
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfirmationToken {
    client_generation: u64,
    server_generation: Option<u64>,
}

#[cfg(test)]
impl ConfirmationToken {
    pub(crate) const fn fixture(client_generation: u64, server_generation: Option<u64>) -> Self {
        Self { client_generation, server_generation }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeKind {
    Live,
    Embedded,
    Fixture,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyContext {
    runtime: Option<RuntimeKind>,
    connected: bool,
    client_generation: u64,
    server_generation: Option<u64>,
    capability_version: Option<u16>,
    authorized_operations: Vec<String>,
    runtime_capabilities: Vec<String>,
    degraded: Vec<(String, String)>,
}

impl SafetyContext {
    pub fn new(
        profile: Option<&RuntimeProfile>,
        connected: bool,
        client_generation: u64,
        server_generation: Option<u64>,
        capabilities: Option<&ActiveCapabilitiesInfo>,
    ) -> Self {
        let runtime = profile.map(|profile| match profile {
            RuntimeProfile::Live { .. } => RuntimeKind::Live,
            RuntimeProfile::Embedded { .. } => RuntimeKind::Embedded,
            RuntimeProfile::Fixture { .. } => RuntimeKind::Fixture,
        });
        Self {
            runtime,
            connected,
            client_generation,
            server_generation,
            capability_version: capabilities.map(|value| value.version),
            authorized_operations: capabilities
                .map(|value| value.authorized_operations.clone())
                .unwrap_or_default(),
            runtime_capabilities: capabilities
                .map(|value| value.runtime.clone())
                .unwrap_or_default(),
            degraded: capabilities
                .map(|value| {
                    value
                        .degraded
                        .iter()
                        .map(|item| (item.id.clone(), "daemon reported degraded".into()))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    pub const fn generation_key(&self) -> (u64, Option<u64>) {
        (self.client_generation, self.server_generation)
    }

    pub fn authorize(&self, plane: ControlPlane, action: &SafetyAction) -> Result<(), String> {
        self.authorization(plane, action).map_err(|error| error.diagnostic_message())
    }

    pub fn authorization(
        &self,
        plane: ControlPlane,
        action: &SafetyAction,
    ) -> Result<(), SafetyDenial> {
        if action.plane != plane {
            return Err(SafetyDenial::new(
                SafetyDenialKind::WrongControlPlane,
                action.required_capability.as_deref(),
                None,
            ));
        }
        let runtime = self.runtime.ok_or_else(|| {
            SafetyDenial::new(
                SafetyDenialKind::InvalidRuntime,
                action.required_capability.as_deref(),
                None,
            )
        })?;
        if action.external_network && runtime == RuntimeKind::Fixture {
            return Err(SafetyDenial::new(
                SafetyDenialKind::ExternalNetworkBlocked,
                action.required_capability.as_deref(),
                None,
            ));
        }
        if !action.requires_daemon_session {
            return Ok(());
        }
        if !self.connected {
            return Err(SafetyDenial::new(
                SafetyDenialKind::Disconnected,
                action.required_capability.as_deref(),
                None,
            ));
        }
        if self.server_generation.is_none() {
            return Err(SafetyDenial::new(
                SafetyDenialKind::GenerationUnknown,
                action.required_capability.as_deref(),
                None,
            ));
        }
        match self.capability_version {
            Some(ACTIVE_CAPABILITIES_VERSION) => {}
            Some(version) => {
                return Err(SafetyDenial::new(
                    SafetyDenialKind::CapabilitiesStale,
                    action.required_capability.as_deref(),
                    Some(format!("version {version}")),
                ));
            }
            None => {
                return Err(SafetyDenial::new(
                    SafetyDenialKind::CapabilitiesUnknown,
                    action.required_capability.as_deref(),
                    None,
                ));
            }
        }
        let Some(capability) = action.required_capability.as_deref() else {
            return Ok(());
        };
        if let Some((_, reason)) = self.degraded.iter().find(|(id, _)| id == capability) {
            return Err(SafetyDenial::new(
                SafetyDenialKind::Degraded,
                Some(capability),
                Some(reason.clone()),
            ));
        }
        let authorized = match action.plane {
            ControlPlane::Operate => &self.authorized_operations,
            ControlPlane::Lab => &self.runtime_capabilities,
        };
        if authorized.iter().any(|item| item == capability) {
            Ok(())
        } else {
            Err(SafetyDenial::new(SafetyDenialKind::Unauthorized, Some(capability), None))
        }
    }

    pub fn begin_confirmation(
        &self,
        plane: ControlPlane,
        action: &SafetyAction,
    ) -> Result<ConfirmationToken, String> {
        if !action.is_destructive() {
            return Err("action does not require destructive confirmation".into());
        }
        self.authorize(plane, action)?;
        Ok(ConfirmationToken {
            client_generation: self.client_generation,
            server_generation: self.server_generation,
        })
    }

    pub fn confirm(
        &self,
        plane: ControlPlane,
        action: &SafetyAction,
        token: ConfirmationToken,
    ) -> Result<(), String> {
        self.confirm_authorization(plane, action, token).map_err(|error| error.diagnostic_message())
    }

    pub fn confirm_authorization(
        &self,
        plane: ControlPlane,
        action: &SafetyAction,
        token: ConfirmationToken,
    ) -> Result<(), SafetyDenial> {
        if token.client_generation != self.client_generation
            || token.server_generation != self.server_generation
        {
            return Err(SafetyDenial::new(
                SafetyDenialKind::GenerationChanged,
                action.required_capability.as_deref(),
                None,
            ));
        }
        self.authorization(plane, action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities(authorized: &[&str], runtime: &[&str]) -> ActiveCapabilitiesInfo {
        let mut value = ActiveCapabilitiesInfo::default();
        value.version = ACTIVE_CAPABILITIES_VERSION;
        value.authorized_operations = authorized.iter().map(|item| (*item).into()).collect();
        value.runtime = runtime.iter().map(|item| (*item).into()).collect();
        value
    }

    fn live_context(generation: u64) -> SafetyContext {
        SafetyContext::new(
            Some(&RuntimeProfile::Live { socket_path: "/tmp/styrene.sock".into() }),
            true,
            generation,
            Some(generation),
            Some(&capabilities(&["rpc.reboot"], &["fixture.scenarios"])),
        )
    }

    #[test]
    fn control_planes_reject_cross_plane_actions() {
        let context = live_context(7);
        assert!(
            context
                .authorize(ControlPlane::Lab, &SafetyAction::operate("rpc.reboot", true))
                .is_err()
        );
        assert!(context.authorize(ControlPlane::Operate, &SafetyAction::live_scenario()).is_err());
    }

    #[test]
    fn operate_actions_require_current_capability() {
        let context = live_context(7);
        assert!(
            context
                .authorize(ControlPlane::Operate, &SafetyAction::operate("rpc.reboot", true))
                .is_ok()
        );
        assert!(
            context
                .authorize(ControlPlane::Operate, &SafetyAction::operate("policy.update", true))
                .is_err()
        );
    }

    #[test]
    fn operator_denials_are_typed_and_hide_capability_identifiers() {
        let denial = live_context(7)
            .authorization(
                ControlPlane::Operate,
                &SafetyAction::operate("network.link_close", true),
            )
            .expect_err("capability set must deny link close");
        assert_eq!(denial.kind, SafetyDenialKind::Unauthorized);
        assert!(!denial.operator_message().contains("network.link_close"));
        assert!(denial.diagnostic_message().contains("network.link_close"));
    }

    #[test]
    fn stale_permission_text_is_operator_safe() {
        let mut stale = capabilities(&["network.announce"], &[]);
        stale.version = ACTIVE_CAPABILITIES_VERSION.saturating_add(1);
        let context = SafetyContext::new(
            Some(&RuntimeProfile::Live { socket_path: "/tmp/styrene.sock".into() }),
            true,
            7,
            Some(7),
            Some(&stale),
        );
        let denial = context
            .authorization(ControlPlane::Operate, &SafetyAction::operate("network.announce", false))
            .expect_err("stale permissions must fail closed");
        assert_eq!(denial.kind, SafetyDenialKind::CapabilitiesStale);
        assert!(!denial.operator_message().contains("network.announce"));
        assert!(!denial.operator_message().contains('2'));
    }

    #[test]
    fn session_scoped_operation_requires_current_session_without_capability() {
        let context = live_context(7);
        assert!(
            context
                .authorization(ControlPlane::Operate, &SafetyAction::operate_session(true))
                .is_ok()
        );
    }

    #[test]
    fn confirmation_expires_across_runtime_generation() {
        let action = SafetyAction::operate("rpc.reboot", true);
        let token = live_context(7)
            .begin_confirmation(ControlPlane::Operate, &action)
            .expect("authorized destructive action");
        let error = live_context(8)
            .confirm(ControlPlane::Operate, &action, token)
            .expect_err("stale confirmation must fail closed");
        assert!(error.contains("generation changed"));
    }

    #[test]
    fn confirmation_rechecks_capability_before_submission() {
        let action = SafetyAction::operate("rpc.reboot", true);
        let token = live_context(7)
            .begin_confirmation(ControlPlane::Operate, &action)
            .expect("authorized destructive action");
        let revoked = SafetyContext::new(
            Some(&RuntimeProfile::Live { socket_path: "/tmp/styrene.sock".into() }),
            true,
            7,
            Some(7),
            Some(&capabilities(&[], &["fixture.scenarios"])),
        );
        let error = revoked
            .confirm(ControlPlane::Operate, &action, token)
            .expect_err("revoked capability must fail closed");
        assert!(error.contains("does not authorize rpc.reboot"));
    }

    #[test]
    fn stale_capability_schema_disables_operate_actions() {
        let mut stale = capabilities(&["rpc.reboot"], &[]);
        stale.version = ACTIVE_CAPABILITIES_VERSION.saturating_add(1);
        let context = SafetyContext::new(
            Some(&RuntimeProfile::Live { socket_path: "/tmp/styrene.sock".into() }),
            true,
            7,
            Some(7),
            Some(&stale),
        );
        let error = context
            .authorize(ControlPlane::Operate, &SafetyAction::operate("rpc.reboot", true))
            .expect_err("stale capabilities must fail closed");
        assert!(error.contains("capabilities are stale"));
    }

    #[test]
    fn fixture_profile_rejects_live_runner_before_networking() {
        let profile = RuntimeProfile::Fixture { fixture: crate::backend::FixtureId::Healthy };
        let context = SafetyContext::new(Some(&profile), true, 1, Some(1), None);
        let error = context
            .authorize(ControlPlane::Lab, &SafetyAction::live_scenario())
            .expect_err("Fixture must not launch live interop");
        assert!(error.contains("cannot open external network"));
    }

    #[test]
    fn fixture_scenarios_use_runtime_capability() {
        let context = live_context(7);
        assert!(context.authorize(ControlPlane::Lab, &SafetyAction::fixture_scenario()).is_ok());
    }
}

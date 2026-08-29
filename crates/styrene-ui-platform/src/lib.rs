//! Renderer-independent contracts for mobile operating-system services.

use std::future::Future;
use std::pin::Pin;

pub type PlatformFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowClass {
    Compact,
    Wide,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowMetrics {
    pub class: WindowClass,
    pub width_css_px: u32,
    pub height_css_px: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Appearance {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Contrast {
    Standard,
    Increased,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionPreference {
    Full,
    Reduced,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccessibilityPreferences {
    pub text_scale_percent: u16,
    pub appearance: Appearance,
    pub contrast: Contrast,
    pub motion: MotionPreference,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EdgeInsets {
    pub top_css_px: u32,
    pub right_css_px: u32,
    pub bottom_css_px: u32,
    pub left_css_px: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlatformGeometry {
    pub insets: PlatformInsets,
    pub keyboard: KeyboardGeometry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformInsets {
    CssEnvironment,
    NativeBridge(EdgeInsets),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardGeometry {
    WebViewManaged { visible: bool },
    NativeBridge { occluded_height_css_px: u32 },
}

/// Advisory process state. Durable correctness must not depend on a final callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationLifecycle {
    Active,
    Inactive,
    Background,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionKind {
    Bluetooth,
    Camera,
    Usb,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationState {
    NotDetermined,
    Granted,
    Denied,
    Restricted,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PermissionStatus {
    pub kind: PermissionKind,
    pub state: AuthorizationState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformSnapshot {
    pub generation: u64,
    pub sequence: u64,
    pub window: WindowMetrics,
    pub accessibility: AccessibilityPreferences,
    pub geometry: PlatformGeometry,
    pub lifecycle: ApplicationLifecycle,
    pub permissions: Vec<PermissionStatus>,
    pub notification_authorization: AuthorizationState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformChange {
    Window(WindowMetrics),
    Accessibility(AccessibilityPreferences),
    Geometry(PlatformGeometry),
    Lifecycle(ApplicationLifecycle),
    Permission(PermissionStatus),
    NotificationAuthorization(AuthorizationState),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformEvent {
    Changed { generation: u64, sequence: u64, change: PlatformChange },
    ResyncRequired { generation: u64, dropped_events: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformApplyResult {
    Applied,
    IgnoredStale,
    ResyncRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformState {
    snapshot: PlatformSnapshot,
}

impl PlatformState {
    #[must_use]
    pub const fn new(snapshot: PlatformSnapshot) -> Self {
        Self { snapshot }
    }

    #[must_use]
    pub const fn snapshot(&self) -> &PlatformSnapshot {
        &self.snapshot
    }

    pub fn replace_snapshot(&mut self, snapshot: PlatformSnapshot) -> PlatformApplyResult {
        if snapshot.generation < self.snapshot.generation
            || (snapshot.generation == self.snapshot.generation
                && snapshot.sequence <= self.snapshot.sequence)
        {
            return PlatformApplyResult::IgnoredStale;
        }

        self.snapshot = snapshot;
        PlatformApplyResult::Applied
    }

    pub fn apply_event(&mut self, event: PlatformEvent) -> PlatformApplyResult {
        match event {
            PlatformEvent::Changed { generation, sequence, change } => {
                if generation != self.snapshot.generation || sequence <= self.snapshot.sequence {
                    return PlatformApplyResult::IgnoredStale;
                }

                self.snapshot.sequence = sequence;
                match change {
                    PlatformChange::Window(window) => self.snapshot.window = window,
                    PlatformChange::Accessibility(accessibility) => {
                        self.snapshot.accessibility = accessibility;
                    }
                    PlatformChange::Geometry(geometry) => self.snapshot.geometry = geometry,
                    PlatformChange::Lifecycle(lifecycle) => self.snapshot.lifecycle = lifecycle,
                    PlatformChange::Permission(permission) => {
                        if let Some(current) = self
                            .snapshot
                            .permissions
                            .iter_mut()
                            .find(|current| current.kind == permission.kind)
                        {
                            *current = permission;
                        } else {
                            self.snapshot.permissions.push(permission);
                        }
                    }
                    PlatformChange::NotificationAuthorization(authorization) => {
                        self.snapshot.notification_authorization = authorization;
                    }
                }
                PlatformApplyResult::Applied
            }
            PlatformEvent::ResyncRequired { generation, .. }
                if generation == self.snapshot.generation =>
            {
                PlatformApplyResult::ResyncRequired
            }
            PlatformEvent::ResyncRequired { .. } => PlatformApplyResult::IgnoredStale,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformFailure {
    pub code: String,
    pub retryable: bool,
}

pub trait PlatformEventStream {
    /// A closed stream returns `None`; lag must be reported as `ResyncRequired` first.
    fn next(&mut self) -> PlatformFuture<'_, Option<PlatformEvent>>;
}

pub trait PlatformService {
    fn snapshot(&self) -> PlatformFuture<'_, Result<PlatformSnapshot, PlatformFailure>>;

    /// Implementations must use bounded buffering and report dropped callbacks.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when the platform cannot create a subscription.
    fn subscribe(&self) -> Result<Box<dyn PlatformEventStream>, PlatformFailure>;

    fn request_permission(
        &self,
        kind: PermissionKind,
    ) -> PlatformFuture<'_, Result<PermissionStatus, PlatformFailure>>;

    fn request_notification_authorization(
        &self,
    ) -> PlatformFuture<'_, Result<AuthorizationState, PlatformFailure>>;
}

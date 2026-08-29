use dioxus::prelude::*;
use serde::Deserialize;
use styrene_ui_platform::{
    AccessibilityPreferences, Appearance, ApplicationLifecycle, AuthorizationState, Contrast,
    KeyboardGeometry, MotionPreference, PermissionKind, PermissionStatus, PlatformApplyResult,
    PlatformChange, PlatformEvent, PlatformEventStream, PlatformFailure, PlatformFuture,
    PlatformGeometry, PlatformInsets, PlatformService, PlatformSnapshot, PlatformState, TextScale,
    WindowClass, WindowMetrics,
};

const BACK_LISTENER: &str = r#"
if (!history.state?.styrenePane) {
    history.replaceState({ styrenePane: "root" }, "", location.href);
}
window.addEventListener("popstate", () => {
    document.getElementById("mobile.platform-back")?.click();
});
"#;

const PLATFORM_SNAPSHOT: &str = r#"
const adapter = window.__styrenePlatformAdapter;
if (!adapter) {
    throw new Error("platform adapter is not subscribed");
}
return adapter.snapshot();
"#;

const PLATFORM_SUBSCRIPTION: &str = r#"
window.__styrenePlatformAdapter?.controller.abort();

const controller = new AbortController();
const signal = controller.signal;
const generation = (window.__styrenePlatformGeneration ?? 0) + 1;
window.__styrenePlatformGeneration = generation;
let sequence = 0;
let inFlight = false;
let pendingKind = null;
let droppedEvents = 0;
let viewportBaseline = {
    width: window.visualViewport?.width ?? window.innerWidth,
    height: window.visualViewport?.height ?? window.innerHeight,
    scale: window.visualViewport?.scale ?? 1,
};

const media = {
    dark: matchMedia("(prefers-color-scheme: dark)"),
    contrast: matchMedia("(prefers-contrast: more)"),
    reducedMotion: matchMedia("(prefers-reduced-motion: reduce)"),
};

const editableFocused = () => {
    const active = document.activeElement;
    return active instanceof HTMLInputElement
        || active instanceof HTMLTextAreaElement
        || active instanceof HTMLSelectElement
        || active?.isContentEditable === true;
};

const lifecycle = () => {
    if (document.visibilityState === "hidden") return "background";
    return "active";
};

const snapshot = () => {
    const viewportWidth = window.visualViewport?.width ?? window.innerWidth;
    const viewportHeight = window.visualViewport?.height ?? window.innerHeight;
    const viewportScale = window.visualViewport?.scale ?? 1;
    if (!editableFocused()
        || Math.abs(viewportBaseline.width - viewportWidth) > 40
        || Math.abs(viewportBaseline.scale - viewportScale) > 0.05) {
        viewportBaseline = {
            width: viewportWidth,
            height: viewportHeight,
            scale: viewportScale,
        };
    }
    return {
        generation,
        sequence,
        widthCssPx: Math.max(0, Math.round(window.innerWidth)),
        heightCssPx: Math.max(0, Math.round(window.innerHeight)),
        wide: matchMedia("(min-width: 52rem)").matches,
        appearance: media.dark.matches ? "dark" : "light",
        contrast: media.contrast.matches ? "increased" : "standard",
        motion: media.reducedMotion.matches ? "reduced" : "full",
        textScalePercent: null,
        lifecycle: lifecycle(),
        keyboardVisible: lifecycle() === "active"
            && editableFocused()
            && viewportBaseline.height - viewportHeight > 80,
    };
};

const publish = async (kind) => {
    if (inFlight) {
        droppedEvents += 1;
        pendingKind = pendingKind === null || pendingKind === kind ? kind : "resync";
        return;
    }

    inFlight = true;
    let nextKind = kind;
    do {
        sequence += 1;
        const message = {
            kind: nextKind,
            droppedEvents,
            snapshot: snapshot(),
        };
        droppedEvents = 0;
        pendingKind = null;
        dioxus.send(message);
        await dioxus.recv();
        nextKind = pendingKind;
    } while (!signal.aborted && nextKind !== null);
    inFlight = false;
};

const listen = (target, event, kind) => {
    target.addEventListener(event, () => publish(kind), { signal });
};

listen(window, "resize", "resync");
listen(document, "visibilitychange", "resync");
listen(document, "focusin", "geometry");
listen(document, "focusout", "geometry");
if (window.visualViewport) {
    listen(window.visualViewport, "resize", "geometry");
    listen(window.visualViewport, "scroll", "geometry");
}
for (const preference of Object.values(media)) {
    listen(preference, "change", "accessibility");
}

window.__styrenePlatformAdapter = { controller, generation, snapshot };
await publish("resync");
await new Promise((resolve) => signal.addEventListener("abort", resolve, { once: true }));
sequence += 1;
dioxus.send({
    kind: "closed",
    droppedEvents: 0,
    snapshot: snapshot(),
});
await dioxus.recv();
"#;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct WebPlatformSnapshot {
    generation: u64,
    sequence: u64,
    width_css_px: u32,
    height_css_px: u32,
    wide: bool,
    appearance: WebAppearance,
    contrast: WebContrast,
    motion: WebMotion,
    text_scale_percent: Option<u16>,
    lifecycle: WebLifecycle,
    keyboard_visible: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum WebAppearance {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum WebContrast {
    Standard,
    Increased,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum WebMotion {
    Full,
    Reduced,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum WebLifecycle {
    Active,
    Inactive,
    Background,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebPlatformMessage {
    kind: WebEventKind,
    dropped_events: u64,
    snapshot: WebPlatformSnapshot,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum WebEventKind {
    Window,
    Accessibility,
    Geometry,
    Lifecycle,
    Resync,
    Closed,
}

#[derive(Clone, Copy, Debug, Default)]
struct WebViewPlatformService;

struct WebViewPlatformEventStream {
    eval: document::Eval,
}

impl WebPlatformSnapshot {
    fn platform_snapshot(self) -> PlatformSnapshot {
        PlatformSnapshot {
            generation: self.generation,
            sequence: self.sequence,
            window: self.window(),
            accessibility: self.accessibility(),
            geometry: self.geometry(),
            lifecycle: self.lifecycle.into(),
            permissions: Vec::new(),
            notification_authorization: AuthorizationState::Unavailable,
        }
    }

    const fn window(self) -> WindowMetrics {
        WindowMetrics {
            class: if self.wide { WindowClass::Wide } else { WindowClass::Compact },
            width_css_px: self.width_css_px,
            height_css_px: self.height_css_px,
        }
    }

    const fn accessibility(self) -> AccessibilityPreferences {
        AccessibilityPreferences {
            text_scale: match self.text_scale_percent {
                Some(percent) => TextScale::Percent(percent),
                None => TextScale::Unavailable,
            },
            appearance: match self.appearance {
                WebAppearance::Light => Appearance::Light,
                WebAppearance::Dark => Appearance::Dark,
            },
            contrast: match self.contrast {
                WebContrast::Standard => Contrast::Standard,
                WebContrast::Increased => Contrast::Increased,
            },
            motion: match self.motion {
                WebMotion::Full => MotionPreference::Full,
                WebMotion::Reduced => MotionPreference::Reduced,
            },
        }
    }

    const fn geometry(self) -> PlatformGeometry {
        PlatformGeometry {
            insets: PlatformInsets::CssEnvironment,
            keyboard: KeyboardGeometry::WebViewManaged { visible: self.keyboard_visible },
        }
    }
}

impl From<WebLifecycle> for ApplicationLifecycle {
    fn from(value: WebLifecycle) -> Self {
        match value {
            WebLifecycle::Active => Self::Active,
            WebLifecycle::Inactive => Self::Inactive,
            WebLifecycle::Background => Self::Background,
        }
    }
}

impl WebPlatformMessage {
    fn platform_event(self) -> Option<PlatformEvent> {
        if self.kind == WebEventKind::Closed {
            return None;
        }
        let generation = self.snapshot.generation;
        if self.dropped_events > 0 || self.kind == WebEventKind::Resync {
            return Some(PlatformEvent::ResyncRequired {
                generation,
                dropped_events: self.dropped_events,
            });
        }

        let sequence = self.snapshot.sequence;
        let change = match self.kind {
            WebEventKind::Window => PlatformChange::Window(self.snapshot.window()),
            WebEventKind::Accessibility => {
                PlatformChange::Accessibility(self.snapshot.accessibility())
            }
            WebEventKind::Geometry => PlatformChange::Geometry(self.snapshot.geometry()),
            WebEventKind::Lifecycle => PlatformChange::Lifecycle(self.snapshot.lifecycle.into()),
            WebEventKind::Resync => unreachable!("resync events return before projection"),
            WebEventKind::Closed => unreachable!("closed events return before projection"),
        };
        Some(PlatformEvent::Changed { generation, sequence, change })
    }
}

impl PlatformEventStream for WebViewPlatformEventStream {
    fn next(&mut self) -> PlatformFuture<'_, Option<PlatformEvent>> {
        Box::pin(async move {
            let message = self.eval.recv::<WebPlatformMessage>().await;
            let _ = self.eval.send(());
            message.ok().and_then(WebPlatformMessage::platform_event)
        })
    }
}

impl PlatformService for WebViewPlatformService {
    fn snapshot(&self) -> PlatformFuture<'_, Result<PlatformSnapshot, PlatformFailure>> {
        Box::pin(async move {
            document::eval(PLATFORM_SNAPSHOT)
                .join::<WebPlatformSnapshot>()
                .await
                .map(WebPlatformSnapshot::platform_snapshot)
                .map_err(|error| platform_eval_failure(&error))
        })
    }

    fn subscribe(&self) -> Result<Box<dyn PlatformEventStream>, PlatformFailure> {
        Ok(Box::new(WebViewPlatformEventStream { eval: document::eval(PLATFORM_SUBSCRIPTION) }))
    }

    fn request_permission(
        &self,
        kind: PermissionKind,
    ) -> PlatformFuture<'_, Result<PermissionStatus, PlatformFailure>> {
        Box::pin(
            async move { Ok(PermissionStatus { kind, state: AuthorizationState::Unavailable }) },
        )
    }

    fn request_notification_authorization(
        &self,
    ) -> PlatformFuture<'_, Result<AuthorizationState, PlatformFailure>> {
        Box::pin(async move { Ok(AuthorizationState::Unavailable) })
    }
}

fn platform_eval_failure(error: &document::EvalError) -> PlatformFailure {
    let (code, retryable) = match error {
        document::EvalError::Finished | document::EvalError::Communication(_) => {
            ("platform_eval_interrupted", true)
        }
        document::EvalError::Unsupported => ("platform_eval_unsupported", false),
        document::EvalError::InvalidJs(_) => ("platform_eval_invalid", false),
        document::EvalError::Serialization(_) => ("platform_eval_incompatible", false),
        _ => ("platform_eval_failed", false),
    };
    PlatformFailure { code: code.into(), retryable }
}

pub fn use_back_navigation() {
    use_effect(move || {
        document::eval(BACK_LISTENER);
    });
}

pub fn use_platform_snapshot() -> Signal<Option<PlatformSnapshot>> {
    let mut snapshot = use_signal(|| None);
    use_drop(move || {
        document::eval("window.__styrenePlatformAdapter?.controller.abort();");
    });
    use_effect(move || {
        spawn(async move {
            loop {
                run_platform_subscription(&mut snapshot).await;
                let _ = document::eval("await new Promise((resolve) => setTimeout(resolve, 250));")
                    .await;
            }
        });
    });
    snapshot
}

async fn run_platform_subscription(snapshot: &mut Signal<Option<PlatformSnapshot>>) {
    let service = WebViewPlatformService;
    let Ok(mut events) = service.subscribe() else {
        return;
    };
    let mut state = None::<PlatformState>;
    let mut resync_required = true;
    while let Some(event) = events.next().await {
        apply_platform_event(&mut state, &mut resync_required, event);

        if resync_required {
            let Ok(current) = service.snapshot().await else {
                continue;
            };
            if let Some(state) = state.as_mut() {
                if state.replace_snapshot(current) == PlatformApplyResult::IgnoredStale {
                    continue;
                }
            } else {
                state = Some(PlatformState::new(current));
            }
            resync_required = false;
        }

        if let Some(state) = &state {
            snapshot.set(Some(state.snapshot().clone()));
        }
    }
}

fn apply_platform_event(
    state: &mut Option<PlatformState>,
    resync_required: &mut bool,
    event: PlatformEvent,
) {
    if *resync_required {
        return;
    }
    let Some(state) = state.as_mut() else {
        *resync_required = true;
        return;
    };
    if state.apply_event(event) == PlatformApplyResult::ResyncRequired {
        *resync_required = true;
    }
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

    fn web_snapshot(generation: u64, sequence: u64) -> WebPlatformSnapshot {
        WebPlatformSnapshot {
            generation,
            sequence,
            width_css_px: 390,
            height_css_px: 844,
            wide: false,
            appearance: WebAppearance::Dark,
            contrast: WebContrast::Increased,
            motion: WebMotion::Reduced,
            text_scale_percent: Some(200),
            lifecycle: WebLifecycle::Active,
            keyboard_visible: true,
        }
    }

    #[test]
    fn snapshot_maps_webview_facts_to_typed_platform_state() {
        let snapshot = web_snapshot(4, 7).platform_snapshot();

        assert_eq!(snapshot.generation, 4);
        assert_eq!(snapshot.window.class, WindowClass::Compact);
        assert_eq!(snapshot.accessibility.appearance, Appearance::Dark);
        assert_eq!(snapshot.accessibility.contrast, Contrast::Increased);
        assert_eq!(snapshot.accessibility.text_scale, TextScale::Percent(200));
        assert_eq!(snapshot.geometry.keyboard, KeyboardGeometry::WebViewManaged { visible: true });
        assert_eq!(snapshot.notification_authorization, AuthorizationState::Unavailable);
    }

    #[test]
    fn dropped_callbacks_require_authoritative_resnapshot() {
        let event = WebPlatformMessage {
            kind: WebEventKind::Geometry,
            dropped_events: 3,
            snapshot: web_snapshot(8, 12),
        }
        .platform_event()
        .expect("geometry event");

        assert_eq!(event, PlatformEvent::ResyncRequired { generation: 8, dropped_events: 3 });
    }

    #[test]
    fn subscription_is_bounded_acknowledged_and_idempotent() {
        assert!(PLATFORM_SUBSCRIPTION.contains("controller.abort"));
        assert!(PLATFORM_SUBSCRIPTION.contains("inFlight"));
        assert!(PLATFORM_SUBSCRIPTION.contains("droppedEvents"));
        assert!(PLATFORM_SUBSCRIPTION.contains("await dioxus.recv()"));
        assert!(PLATFORM_SUBSCRIPTION.contains("kind: \"closed\""));
        assert!(PLATFORM_SUBSCRIPTION.contains("prefers-color-scheme"));
        assert!(PLATFORM_SUBSCRIPTION.contains("prefers-contrast"));
        assert!(PLATFORM_SUBSCRIPTION.contains("prefers-reduced-motion"));
        assert!(PLATFORM_SUBSCRIPTION.contains("visibilitychange"));
        assert!(PLATFORM_SUBSCRIPTION.contains("visualViewport"));
    }

    #[test]
    fn webview_payload_deserializes_exact_wire_names() {
        let payload = r#"{
            "kind": "accessibility",
            "droppedEvents": 0,
            "snapshot": {
                "generation": 2,
                "sequence": 5,
                "widthCssPx": 390,
                "heightCssPx": 844,
                "wide": false,
                "appearance": "dark",
                "contrast": "increased",
                "motion": "reduced",
                "textScalePercent": null,
                "lifecycle": "active",
                "keyboardVisible": false
            }
        }"#;

        let message: WebPlatformMessage =
            serde_json::from_str(payload).expect("webview payload must deserialize");

        assert_eq!(message.kind, WebEventKind::Accessibility);
        assert_eq!(message.snapshot.text_scale_percent, None);
        assert_eq!(message.snapshot.appearance, WebAppearance::Dark);
    }

    #[test]
    fn failed_resnapshot_blocks_later_partial_events() {
        let initial = web_snapshot(3, 7).platform_snapshot();
        let mut state = Some(PlatformState::new(initial.clone()));
        let mut resync_required = true;

        apply_platform_event(
            &mut state,
            &mut resync_required,
            PlatformEvent::Changed {
                generation: 3,
                sequence: 8,
                change: PlatformChange::Lifecycle(ApplicationLifecycle::Background),
            },
        );

        assert_eq!(state.expect("state").snapshot(), &initial);
        assert!(resync_required);
    }
}

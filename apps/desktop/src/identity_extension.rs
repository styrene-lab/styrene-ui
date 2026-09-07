//! Concrete host adapter for one optionally compiled Identity page.
//! Mesh owns routes, session context, and its enable preference. The client has
//! no custody, arbitrary daemon-call, raw path, or CLI execution capability.

use crate::{components::workspaces::Destination, state::AppRoute, stores::RuntimeStore};
use dioxus::prelude::*;

#[cfg(not(feature = "identity-extension"))]
pub fn use_host(
    items: Vec<Destination>,
    _: Signal<AppRoute>,
    _: &RuntimeStore,
    _: Option<&styrene_ipc::types::IdentityInfo>,
    _: bool,
) -> Vec<Destination> {
    items
}
#[cfg(not(feature = "identity-extension"))]
#[component]
pub fn Controls() -> Element {
    rsx! {}
}
#[cfg(not(feature = "identity-extension"))]
#[component]
pub fn Page(
    generation: u64,
    server_generation: Option<u64>,
    connected: bool,
    fixture: bool,
    transport_identity: Option<String>,
) -> Element {
    let _ = (generation, server_generation, connected, fixture, transport_identity);
    rsx! {section{class:"state-banner","The optional Identity component is not bundled. Core Mesh controls remain available."}}
}

#[cfg(feature = "identity-extension")]
mod enabled {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::{
        io::{Read, Write},
        path::{Path, PathBuf},
        sync::Arc,
    };
    use styrene_identity::overview::*;
    use styrene_identity_ui::{
        IDENTITY_EXTENSION, IdentityExtension, IdentityPage, OverviewClient,
    };

    const HOST_REVISION: u32 = 1;
    #[derive(Clone, Debug, PartialEq)]
    struct SessionContext {
        generation: u64,
        server_generation: Option<u64>,
        connected: bool,
        fixture: bool,
        transport_identity: Option<String>,
    }

    struct PublicClient {
        context: SessionContext,
    }
    #[async_trait::async_trait]
    impl IdentityOverviewSource for PublicClient {
        async fn read_public_overview(
            &self,
            selection: IdentitySelection,
        ) -> Result<IdentityOverview, OverviewError> {
            if selection.0 != self.context.generation {
                return Err(OverviewError::Unavailable(UnavailableReason::NotKnown));
            }
            if !self.context.connected {
                return Err(OverviewError::Unavailable(UnavailableReason::ProviderUnavailable));
            }
            if !self.context.fixture {
                // Current IdentityInfo contains RNS/LXMF addresses, not the
                // canonical Identity signing key. Never manufacture that binding.
                return Err(OverviewError::Unavailable(UnavailableReason::Unsupported));
            }
            Ok(IdentityOverview {
                identity: PublicIdentityStatus::Available(PublicIdentitySummary::from_public_key(
                    [42; 32],
                )),
                availability: ProviderAvailability::Unknown,
                root_exposure: RootExposure::Unknown,
                capabilities: vec![CapabilitySummary {
                    capability: IdentityCapability::PublicOverview,
                    availability: ProviderAvailability::Available,
                }],
            })
        }
    }

    #[derive(Clone)]
    struct Host {
        model: IdentityExtension,
        context: SessionContext,
        client: OverviewClient,
        path: Option<PathBuf>,
        ready: bool,
        saving: bool,
        error: Option<String>,
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Preference {
        schema_version: u32,
        enabled: bool,
    }

    fn preference_path() -> Result<PathBuf, String> {
        let root = std::env::var_os("STYRENE_DX_SETTINGS_ROOT")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .map(|p| p.join("styrene-desktop"))
            })
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|p| p.join(".config/styrene-desktop"))
            })
            .filter(|p| p.is_absolute())
            .ok_or_else(|| "An absolute host settings directory is unavailable.".to_owned())?;
        Ok(root.join("identity-extension.json"))
    }

    fn load_preference(path: &Path) -> Result<Option<bool>, String> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("Identity extension preference cannot be read.".into()),
        };
        if !file.metadata().map_err(|_| "Preference metadata unavailable")?.is_file() {
            return Err("Identity preference is not a regular file.".into());
        }
        let mut bytes = Vec::new();
        file.take(4097).read_to_end(&mut bytes).map_err(|_| "Preference read failed")?;
        if bytes.len() > 4096 {
            return Err("Identity extension preference is oversized.".into());
        }
        let value: Preference = serde_json::from_slice(&bytes)
            .map_err(|_| "Identity extension preference is invalid")?;
        if value.schema_version != 1 {
            return Err("Identity extension preference version is unsupported.".into());
        }
        Ok(Some(value.enabled))
    }

    fn save_preference(path: &Path, enabled: bool) -> Result<(), String> {
        let parent = path.parent().ok_or_else(|| "Preference destination is invalid".to_owned())?;
        std::fs::create_dir_all(parent).map_err(|_| "Preference directory cannot be created")?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| "Preference staging failed")?;
        let bytes = serde_json::to_vec(&Preference { schema_version: 1, enabled })
            .map_err(|_| "Preference encoding failed")?;
        temporary
            .write_all(&bytes)
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|_| "Preference write failed")?;
        temporary.persist(path).map_err(|_| "Preference commit failed")?;
        #[cfg(unix)]
        std::fs::File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|_| "Preference directory sync failed")?;
        Ok(())
    }

    pub fn use_host(
        mut items: Vec<Destination>,
        route: Signal<AppRoute>,
        runtime: &RuntimeStore,
        identity: Option<&styrene_ipc::types::IdentityInfo>,
        fixture: bool,
    ) -> Vec<Destination> {
        let current = SessionContext {
            generation: runtime.generation.0,
            server_generation: runtime.server_generation,
            connected: runtime.connected,
            fixture,
            transport_identity: identity.map(|id| id.identity_hash.clone()),
        };
        let initial = current.clone();
        let mut host = use_signal(move || Host {
            model: IdentityExtension::new(true, IDENTITY_EXTENSION, HOST_REVISION, Some(false)),
            client: OverviewClient::new(Arc::new(PublicClient { context: initial.clone() })),
            context: initial,
            path: None,
            ready: false,
            saving: false,
            error: None,
        });
        use_context_provider(|| host);
        use_context_provider(|| route);
        use_future(move || async move {
            let loaded = tokio::task::spawn_blocking(|| {
                let path = preference_path()?;
                let value = load_preference(&path)?;
                Ok::<_, String>((path, value))
            })
            .await;
            let mut state = host.write();
            state.ready = true;
            match loaded {
                Ok(Ok((path, value))) => {
                    state.path = Some(path);
                    state.model =
                        IdentityExtension::new(true, IDENTITY_EXTENSION, HOST_REVISION, value);
                }
                Ok(Err(error)) => state.error = Some(error),
                Err(_) => state.error = Some("Identity preference worker failed.".into()),
            }
        });
        use_effect(use_reactive((&current,), move |(context,)| {
            let mut state = host.write();
            if state.context != context {
                let _ = state.model.session_changed();
                state.client =
                    OverviewClient::new(Arc::new(PublicClient { context: context.clone() }));
                state.context = context;
            }
        }));
        if host.read().ready && host.read().model.registered() {
            items.push(Destination {
                route: AppRoute::Identity,
                label: IDENTITY_EXTENSION.title,
                summary: "Optional Identity management".into(),
                tools: true,
                icon: "M12 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8 M4 21v-3a8 8 0 0 1 16 0v3",
            });
        }
        // A direct route hint cannot leave the host on a disabled page.
        use_effect(move || {
            let disabled = host.read().ready && !host.read().model.registered();
            if disabled && *route.peek() == AppRoute::Identity {
                let mut route = route;
                route.set(AppRoute::Overview);
            }
        });
        items
    }

    #[component]
    pub fn Controls() -> Element {
        let mut host = use_context::<Signal<Host>>();
        let mut route = use_context::<Signal<AppRoute>>();
        let enabled = host.read().model.enabled_preference();
        let disabled = !host.read().ready || host.read().saving || host.read().path.is_none();
        rsx! {div{class:"identity-extension-controls",
            label{title:"Enable the optional Identity page; this does not unlock custody.",
                input{r#type:"checkbox",checked:enabled,disabled,onchange:move|event|{
                    let desired=event.checked();let path=host.read().path.clone();
                    if host.read().saving||desired==host.read().model.enabled_preference(){return}
                    let Some(path)=path else{return};host.write().saving=true;
                    spawn(async move{
                        let saved=tokio::task::spawn_blocking(move||save_preference(&path,desired)).await;
                        let mut state=host.write();state.saving=false;
                        match saved {
                            Ok(Ok(()))=>{state.error=None;if desired{let _=state.model.enable();}else{let _=state.model.disable(false,true);if *route.peek()==AppRoute::Identity{route.set(AppRoute::Overview);}}},
                            _=>state.error=Some("Identity enable preference was not saved; state was preserved.".into()),
                        }
                    });
                }}
                "Identity tools"
            }
            if let Some(error)=host.read().error.as_ref(){span{role:"status","{error}"}}
        }}
    }

    #[component]
    pub fn Page(
        generation: u64,
        server_generation: Option<u64>,
        connected: bool,
        fixture: bool,
        transport_identity: Option<String>,
    ) -> Element {
        let mut host = use_context::<Signal<Host>>();
        use_effect(move || {
            let mount = {
                let state = host.read();
                state.ready && state.model.registered() && !state.model.mounted()
            };
            if mount {
                let _ = host.write().model.mount();
            }
        });
        use_drop(move || {
            if let Ok(mut state) = host.try_write() {
                let _ = state.model.unmount();
            }
        });
        let state = host.read().clone();
        let current = SessionContext {
            generation,
            server_generation,
            connected,
            fixture,
            transport_identity,
        };
        if state.context != current {
            return rsx! {p{role:"status","Updating Identity context for the selected session…"}};
        }
        if !state.model.registered() || !state.model.mounted() {
            return rsx! {p{role:"status","Identity page is unavailable or initializing."}};
        }
        let request = OverviewRequest {
            selection: IdentitySelection(state.context.generation),
            scope: OverviewScope(state.model.generation()),
            expected_identity: None,
        };
        rsx! {section{class:"identity-extension-page",key:"{state.context.generation}:{state.model.generation()}",
            p{class:"system-note","Identity component {IDENTITY_EXTENSION.build_version} · host contract {HOST_REVISION}. Mesh session and profile ownership stay with the host."}
            if !state.context.fixture {p{class:"system-note","The current daemon contract supplies transport identifiers but no canonical Identity public key. The provider reports unsupported rather than substituting an RNS or LXMF address."}}
            IdentityPage{client:state.client,request,fixture:state.context.fixture}
        }}
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn preference_roundtrip_stores_only_enablement() {
            let dir = tempfile::tempdir().expect("temporary settings directory");
            let path = dir.path().join("preferences.json");
            assert_eq!(load_preference(&path).expect("absent preference"), None);
            save_preference(&path, false).expect("save disabled preference");
            assert_eq!(load_preference(&path).expect("read saved preference"), Some(false));
            let text = std::fs::read_to_string(path).expect("read preference JSON");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&text).expect("valid preference JSON"),
                serde_json::json!({"schema_version":1,"enabled":false})
            );
            let model =
                IdentityExtension::new(true, IDENTITY_EXTENSION, HOST_REVISION, Some(false));
            assert!(!model.registered());
            assert!(!model.mounted());
        }
        #[tokio::test]
        async fn live_transport_identity_is_not_substituted_for_canonical_identity() {
            let source = PublicClient {
                context: SessionContext {
                    generation: 1,
                    server_generation: Some(1),
                    connected: true,
                    fixture: false,
                    transport_identity: Some("ab".repeat(16)),
                },
            };
            assert!(matches!(
                source.read_public_overview(IdentitySelection(1)).await,
                Err(OverviewError::Unavailable(UnavailableReason::Unsupported))
            ));
        }
        #[tokio::test]
        async fn fixture_is_explicit_and_generation_changes_invalidate_old_results() {
            let mut model = IdentityExtension::new(true, IDENTITY_EXTENSION, 1, None);
            let old = model.mount().expect("mount compatible extension");
            let source = PublicClient {
                context: SessionContext {
                    generation: 1,
                    server_generation: Some(1),
                    connected: true,
                    fixture: true,
                    transport_identity: None,
                },
            };
            let result =
                source.read_public_overview(IdentitySelection(1)).await.expect("fixture overview");
            assert!(matches!(result.identity, PublicIdentityStatus::Available(_)));
            model.session_changed().expect("invalidate previous session");
            assert!(!model.accepts_view_result(old));
            model.disable(false, true).expect("disable idle extension");
            assert!(!model.registered());
            model.enable().expect("enable compatible extension");
            assert!(model.registered());
            assert!(!model.mounted());
        }
    }
}

#[cfg(feature = "identity-extension")]
pub use enabled::{Controls, Page, use_host};

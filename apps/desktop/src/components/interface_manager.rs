use crate::{daemon_bridge::DaemonCommand, state::InterfaceInfo};
use dioxus::prelude::*;
use styrene_ipc::types::{InterfaceInventory, InterfaceMutation, InterfaceSettings};

fn type_label(kind: &str) -> &str {
    match kind {
        "tcp_client" => "TCP client",
        "tcp_server" => "TCP listener",
        "rnode" => "RNode",
        other => other,
    }
}
fn fresh() -> InterfaceSettings {
    let mut s = InterfaceSettings::default();
    s.kind = "tcp_client".into();
    s.port = 4242;
    s.enabled = true;
    s
}
fn request(revision: String, action: &str, settings: InterfaceSettings) -> InterfaceMutation {
    let mut r = InterfaceMutation::default();
    r.expected_revision = revision;
    r.action = action.into();
    r.settings = settings;
    r
}
#[component]
pub fn InterfaceManager(
    inventory: Option<InterfaceInventory>,
    load_error: Option<String>,
    mutation_error: ReadSignal<Option<String>>,
    result_serial: ReadSignal<u64>,
    action_notice: Option<String>,
    focus: ReadSignal<Option<String>>,
    unavailable: Option<String>,
    connected: ReadSignal<bool>,
    generation: ReadSignal<(u64, Option<u64>)>,
    runtime: Vec<InterfaceInfo>,
    on_command: EventHandler<DaemonCommand>,
) -> Element {
    let mut selected = use_signal(|| None::<String>);
    let mut draft = use_signal(|| None::<InterfaceSettings>);
    let mut port = use_signal(|| "4242".to_string());
    let mut busy = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);
    let mut search = use_signal(String::new);
    let mut submitted_revision = use_signal(String::new);
    use_effect(move || {
        let _ = generation();
        let _ = connected();
        selected.set(None);
        draft.set(None);
        busy.set(false);
        confirm_delete.set(false);
    });
    use_effect(move || {
        let serial = result_serial();
        if serial > 0 {
            busy.set(false);
            if mutation_error.peek().is_none() {
                draft.set(None);
                confirm_delete.set(false);
                selected.set(focus.peek().clone());
            }
        }
    });
    let available =
        connected() && inventory.is_some() && load_error.is_none() && unavailable.is_none();
    let entries = inventory.as_ref().map(|i| i.entries.clone()).unwrap_or_default();
    let revision = inventory.as_ref().map(|i| i.revision.clone()).unwrap_or_default();
    let chosen = entries.iter().find(|e| Some(&e.settings.id) == selected.read().as_ref()).cloned();
    let editing = draft.read().clone();
    let is_editing = editing.is_some();
    let inspecting = chosen.is_some() || is_editing;
    let filter = search.read().to_lowercase();
    let visible: Vec<_> = entries
        .iter()
        .filter(|e| {
            format!("{} {} {}", e.settings.name, e.settings.kind, e.settings.host)
                .to_lowercase()
                .contains(&filter)
        })
        .cloned()
        .collect();
    let stale = is_editing
        && !submitted_revision.read().is_empty()
        && *submitted_revision.read() != revision;
    let locked = !available || busy();
    rsx! {
        section { class: "interface-manager",
            div { class: "interface-toolbar",
                div {
                    h3 { "Your interfaces" }
                    p { "Connection state and traffic refresh every 2 seconds. Disabled interfaces remain listed." }
                }
                button { class: "primary", disabled: locked || is_editing, onclick: move |_| {
                    selected.set(None);draft.set(Some(fresh()));port.set("4242".into());
                    submitted_revision.set(revision.clone());confirm_delete.set(false);
                }, "+ Add interface" }
            }
            if !connected() { p { class:"interface-error", role:"status", "Reconnect to the daemon to manage interfaces." } }
            if let Some(error)=load_error { p { class:"interface-error", role:"alert", "Interface management unavailable: {error}" } }
            if let Some(error)=unavailable { p { class:"interface-note", "{error}" } }
            if let Some(error)=mutation_error() { p { class:"interface-error", role:"alert", "{error}" } }
            if busy() { p { class:"interface-note", role:"status", "Applying interface change…" } }
            else if let Some(message)=action_notice { p { class:"interface-note", role:"status", "{message}" } }
            div { class: if inspecting { "interface-layout inspecting" } else { "interface-layout" },
                div { class:"interface-list",
                    input { aria_label:"Search interfaces", placeholder:"Search name, type, or endpoint…", value:"{search}", oninput:move |e| search.set(e.value()) }
                    if entries.is_empty() && inventory.is_some() { div { class:"network-empty", h3 { "No interfaces configured" } p { "Add a TCP connection or listener to begin." } } }
                    else if visible.is_empty() && inventory.is_some() { p { "No matching interfaces." } }
                    else {
                        div { class:"interface-table-scroll",
                            table { class:"interface-table",
                                thead { tr { th { "Name" } th { "Type" } th { "Endpoint" } th { "Enabled" } th { "State" } th { "Received" } th { title:"Bytes queued to the interface for transmission", "TX queued" } } }
                                tbody {
                                    for entry in visible {
                                        {
                                            let id=entry.settings.id.clone();
                                            let active=selected.read().as_ref()==Some(&id);
                                            rsx! { tr { class:if active {"selected"}else{""},
                                                td { button { disabled:busy() || is_editing, onclick:move |_| {selected.set(Some(id.clone()));confirm_delete.set(false);}, "{entry.settings.name}" } }
                                                td { "{type_label(&entry.settings.kind)}" }
                                                td { "{entry.settings.host}:{entry.settings.port}" }
                                                td { if entry.settings.enabled {"Yes"} else {"No"} }
                                                td { "{entry.state}" if let Some(error)=&entry.error { p { class:"interface-row-error", "{error}" } } }
                                                td { "{entry.rx_bytes} B" }
                                                td { "{entry.tx_bytes} B" }
                                            } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(settings)=editing {
                    aside { class:"interface-inspector",
                        h3 { if settings.id.is_empty() {"Add interface"} else {"Edit interface"} }
                        if stale { p { class:"interface-error", "Configuration changed while editing. Cancel and reopen the editor to use the latest settings." } }
                        form { onsubmit: move |event| {
                            event.prevent_default();
                            if locked || stale {return;}
                            let Ok(number)=port.read().parse::<u16>() else {return;};
                            if let Some(mut settings)=draft.read().clone() {
                                settings.port=number;
                                let action=if settings.id.is_empty(){"create"}else{"update"};
                                busy.set(true);
                                on_command.call(DaemonCommand::MutateInterface(request(submitted_revision.read().clone(),action,settings)));
                            }
                        },
                            label { "Type"
                                select { value:"{settings.kind}", disabled:locked || !settings.id.is_empty(), onchange:move |e| {
                                    draft.with_mut(|d| {if let Some(s)=d {s.kind=e.value();s.host=if s.kind=="tcp_server" {"127.0.0.1".into()}else{String::new()};}});
                                }, option {value:"tcp_client","TCP client"} option {value:"tcp_server","TCP listener"} }
                            }
                            p { class:"interface-note", if settings.kind=="tcp_client" { "Connects to another node. Failed connections retry automatically." } else { "Accepts incoming connections. Use 127.0.0.1 for local access or 0.0.0.0 for all IPv4 interfaces." } }
                            label { "Name" input { required:true, maxlength:80, value:"{settings.name}", disabled:locked, oninput:move |e| draft.with_mut(|d| {if let Some(s)=d{s.name=e.value();}}) } }
                            label { if settings.kind=="tcp_client" {"Remote host"}else{"Bind address"}
                                input { required:true, maxlength:253, value:"{settings.host}", placeholder:if settings.kind=="tcp_client" {"rns.styrene.io"}else{"127.0.0.1"}, disabled:locked, oninput:move |e| draft.with_mut(|d| {if let Some(s)=d{s.host=e.value();}}) }
                            }
                            label { "Port" input { r#type:"number", min:if settings.kind=="tcp_client" {"1"}else{"0"}, max:"65535", required:true, value:"{port}", disabled:locked, oninput:move |e| port.set(e.value()) } }
                            if settings.kind=="tcp_server" { p { class:"interface-note", "Port 0 requests an automatically assigned port." } }
                            label { class:"interface-checkbox", input {r#type:"checkbox", checked:settings.enabled, disabled:locked, onchange:move |e| draft.with_mut(|d| {if let Some(s)=d{s.enabled=e.checked();}})}, "Enable now and on daemon startup" }
                            if !settings.id.is_empty() { p { class:"interface-note", "Applying changes reconnects this interface. Editing a listener closes its accepted connections." } }
                            div { class:"interface-buttons",
                                button { r#type:"submit", class:"primary", disabled:locked || stale || port.read().parse::<u16>().is_err(), if busy(){"Applying…"}else if settings.id.is_empty() && settings.enabled {"Save and enable"} else if settings.id.is_empty(){"Save disabled"}else{"Apply"} }
                                button { r#type:"button", disabled:busy(), onclick:move |_| {draft.set(None);confirm_delete.set(false);}, "Cancel" }
                            }
                        }
                        p { class:"interface-note", "Radio configuration requires a runtime with a supported radio management adapter." }
                    }
                } else if let Some(entry)=chosen {
                    {
                        let edit_settings=entry.settings.clone();let toggle_settings=entry.settings.clone();let retry_settings=entry.settings.clone();let delete_settings=entry.settings.clone();
                        let edit_revision=inventory.as_ref().map(|i|i.revision.clone()).unwrap_or_default();let toggle_revision=edit_revision.clone();let retry_revision=edit_revision.clone();let delete_revision=edit_revision.clone();
                        let can_act=!locked && entry.editable;
                        let children:Vec<_>=runtime.iter().filter(|r|r.parent_hash.is_some() && r.parent_hash==entry.runtime_hash).cloned().collect();
                        rsx! { aside { class:"interface-inspector",
                            div { class:"interface-inspector-heading", h3 {"{entry.settings.name}"} button {disabled:busy(),onclick:move |_|{selected.set(None);confirm_delete.set(false);},"Close"} }
                            p { "{type_label(&entry.settings.kind)} · {entry.state}" }
                            if let Some(error)=entry.error { p {class:"interface-error",role:"alert","{error}"} }
                            if !entry.editable { p {class:"interface-note","This runtime reports this interface but does not support editing it."} }
                            dl { dt {"Configured endpoint"} dd {"{entry.settings.host}:{entry.settings.port}"}
                                dt {"Local endpoint"} dd {{entry.local_endpoint.as_deref().unwrap_or("Not reported")}}
                                dt {"Remote endpoint"} dd {{entry.remote_endpoint.as_deref().unwrap_or("Not reported")}}
                                dt {"TX queued / received"} dd {"{entry.tx_bytes} B / {entry.rx_bytes} B"}
                            }
                            div {class:"interface-buttons",
                                button {disabled:!can_act,onclick:move |_|{port.set(edit_settings.port.to_string());draft.set(Some(edit_settings.clone()));submitted_revision.set(edit_revision.clone());},"Edit"}
                                button {disabled:!can_act,onclick:move |_|{let mut s=toggle_settings.clone();s.enabled = !s.enabled;busy.set(true);on_command.call(DaemonCommand::MutateInterface(request(toggle_revision.clone(),"update",s)));},if entry.settings.enabled {"Disable"}else{"Enable"}}
                                button {disabled:!can_act || !entry.settings.enabled,onclick:move |_|{busy.set(true);on_command.call(DaemonCommand::MutateInterface(request(retry_revision.clone(),"reconnect",retry_settings.clone())));},if entry.settings.kind=="tcp_server"{"Restart listener"}else{"Reconnect"}}
                            }
                            if entry.settings.kind=="tcp_server" {
                                h4 {"Accepted connections"}
                                if children.is_empty() {p {"No accepted connections reported."}}
                                for child in children { p {{format!("{} · {}", child.remote_endpoint.as_deref().unwrap_or("Endpoint unknown"), child.status)}} }
                            }
                            details { summary {"Diagnostics"} p {class:"interface-id","Configuration ID: {entry.settings.id}"} p {class:"interface-id",{format!("Runtime ID: {}", entry.runtime_hash.as_deref().unwrap_or("None"))}} }
                            if confirm_delete() {
                                div {class:"interface-delete-confirm",role:"alert",
                                    p {"Delete {entry.settings.name}? This removes its saved configuration and stops the interface and its accepted connections."}
                                    button {class:"danger",disabled:!can_act,onclick:move |_|{busy.set(true);on_command.call(DaemonCommand::MutateInterface(request(delete_revision.clone(),"delete",delete_settings.clone())));},"Delete interface"}
                                    button {disabled:busy(),onclick:move |_|confirm_delete.set(false),"Keep interface"}
                                }
                            } else { button {class:"danger",disabled:!can_act,onclick:move |_|confirm_delete.set(true),"Delete…"} }
                        } }
                    }
                }
            }
        }
    }
}

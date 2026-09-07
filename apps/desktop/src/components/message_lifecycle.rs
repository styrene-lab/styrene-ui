use dioxus::prelude::*;

use crate::state::ChatMessage;

#[component]
pub fn MessageBubble(message: ChatMessage) -> Element {
    let mut details_open = use_signal(|| false);
    let status_label = match message.lifecycle.state {
        styrene_ipc::types::MessageLifecycleState::Delivered => "Delivered",
        styrene_ipc::types::MessageLifecycleState::Failed => "Failed",
        styrene_ipc::types::MessageLifecycleState::Expired => "Expired",
        styrene_ipc::types::MessageLifecycleState::Rejected => "Rejected",
        styrene_ipc::types::MessageLifecycleState::Queued => "Queued",
        styrene_ipc::types::MessageLifecycleState::Sending => "Sending…",
        styrene_ipc::types::MessageLifecycleState::Sent => "Sent",
        _ => "Delivery status",
    };
    rsx! {
        article {
            class: if message.is_outgoing { "message-item outgoing" } else { "message-item incoming" },
            "data-message-id": message.id.clone(),
            "data-outgoing": message.is_outgoing.to_string(),
            div { class: if message.is_outgoing { "message sent" } else { "message received" },
                if !message.content.is_empty() {
                    div { class: "message-content", "{message.content}" }
                }
                if !message.lifecycle.attachments.is_empty() {
                    div { class: "message-attachments", aria_label: "Attachments",
                        for attachment in &message.lifecycle.attachments {
                            div { class: "message-attachment", key: "{attachment.ordinal}",
                                span { class: "attachment-file-icon", aria_hidden: "true", "▤" }
                                div {
                                    strong { "{attachment.name}" }
                                    span { "{attachment_size(attachment.size)} · {attachment_availability(&attachment.availability)}" }
                                }
                            }
                        }
                    }
                }
            }
            footer { class: "message-caption",
                time { "data-message-time": message.timestamp.to_string(), "…" }
                button {
                    class: "message-details-link",
                    r#type: "button",
                    aria_haspopup: "dialog",
                    aria_label: if message.is_outgoing { format!("{status_label}. Delivery details") } else { "Message details".into() },
                    onclick: move |_| details_open.set(true),
                    if message.is_outgoing { "{status_label} · Details" } else { "Details" }
                }
            }
            if details_open() {
                dialog {
                    class: "message-detail-dialog",
                    aria_label: "Message delivery details",
                    onmounted: move |_| {
                        spawn(async move {
                            let _ = document::eval(r#"
                                const dialog = document.querySelector('.message-detail-dialog:not([open])');
                                if (dialog) {
                                    await new Promise(resolve => {
                                        const finish = () => { removal.disconnect(); resolve(); };
                                        const removal = new MutationObserver(() => { if (!dialog.isConnected) finish(); });
                                        removal.observe(document.body, {childList:true, subtree:true});
                                        dialog.addEventListener('close', finish, {once:true});
                                        dialog.showModal();
                                    });
                                }
                                return null;
                            "#).join::<()>().await;
                            details_open.set(false);
                        });
                    },
                    header {
                        h2 { "Delivery details" }
                        button {
                            r#type: "button", autofocus: true, aria_label: "Close delivery details",
                            onclick: move |_| {
                                spawn(async move {
                                    let _ = document::eval("document.querySelector('.message-detail-dialog[open]')?.close();").join::<()>().await;
                                });
                            },
                            "Close"
                        }
                    }
                div { class: "message-lifecycle-grid",
                    Field { label: "Status", value: message.status.clone() }
                    Field { label: "Lifecycle", value: format!("{:?}", message.lifecycle.state) }
                    Field {
                        label: "Terminal detail",
                        value: message.lifecycle.terminal_detail.clone().unwrap_or_else(|| "Not reported".into()),
                    }
                    Field { label: "Authenticity", value: format!("{:?}", message.lifecycle.authentication) }
                    Field { label: "Stamp state", value: format!("{:?}", message.lifecycle.stamp_state) }
                    Field {
                        label: "Stamp cost",
                        value: message.lifecycle.stamp_cost.map(|value| value.to_string()).unwrap_or_else(|| "Not reported".into()),
                    }
                    Field {
                        label: "Stamp value",
                        value: message.lifecycle.stamp_value.map(|value| value.to_string()).unwrap_or_else(|| "Not reported".into()),
                    }
                    Field {
                        label: "Requested method",
                        value: message.lifecycle.requested_method.clone().unwrap_or_else(|| "Not reported".into()),
                    }
                    Field {
                        label: "Actual method",
                        value: message.lifecycle.actual_method.clone().unwrap_or_else(|| "Not reported".into()),
                    }
                    if message.lifecycle.evidence.is_empty() {
                        Field { label: "Delivery evidence", value: String::from("Not reported") }
                    }
                    for evidence in &message.lifecycle.evidence {
                        Field {
                            label: "Delivery evidence",
                            value: format!(
                                "kind={:?} hash={} representation={} state={:?} outcome={} attempt={} correlation={} observed={} terminal={}",
                                evidence.kind,
                                evidence.hash,
                                evidence.representation,
                                evidence.state,
                                evidence.outcome.as_deref().unwrap_or("Not reported"),
                                evidence.attempt.map(|value| value.to_string()).unwrap_or_else(|| "Not reported".into()),
                                evidence.correlation_id.as_deref().unwrap_or("Not reported"),
                                evidence.observed_at,
                                evidence.terminal_at.map(|value| value.to_string()).unwrap_or_else(|| "Not reported".into()),
                            ),
                        }
                    }
                    if message.lifecycle.attachments.is_empty() {
                        Field { label: "Attachments", value: String::from("Not reported") }
                    }
                    for attachment in &message.lifecycle.attachments {
                        Field {
                            label: "Attachment",
                            value: format!("{} size={} checksum={} integrity={} availability={}", attachment.name, attachment.size, attachment.checksum, attachment.integrity, attachment.availability),
                        }
                        if let Some(transfer) = attachment.transfer.as_deref() {
                            Field {
                                label: "Transfer",
                                value: format!("resource={} progress={}/{} checksum_verified={} cancellable={} state={} error={}", transfer.resource_hash.as_deref().unwrap_or("Not reported"), transfer.transferred, transfer.total, transfer.checksum_verified, transfer.cancellable, transfer.state, transfer.error.as_deref().unwrap_or("Not reported")),
                            }
                        }
                    }
                    if message.lifecycle.propagation.is_empty() {
                        Field { label: "Propagation", value: String::from("Not reported") }
                    }
                    for correlation in &message.lifecycle.propagation {
                        Field {
                            label: "Propagation",
                            value: format!("relation={} transient={} attempt={} peer={} state={} created={} updated={}", correlation.relation, correlation.transient_id, correlation.attempt_id.as_deref().unwrap_or("Not reported"), correlation.peer_hash.as_deref().unwrap_or("Not reported"), correlation.state, correlation.created_at, correlation.updated_at),
                        }
                    }
                }
                }
            }
        }
    }
}

fn attachment_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{} KiB", bytes.div_ceil(1024))
    } else {
        format!("{} MiB", bytes.div_ceil(1024 * 1024))
    }
}

fn attachment_availability(value: &str) -> &'static str {
    match value {
        "available" => "Available on this device",
        "unavailable" => "Not available on this device",
        _ => "Availability unknown",
    }
}

#[component]
fn Field(label: &'static str, value: String) -> Element {
    rsx! {
        div {
            span { "{label}" }
            code { "{value}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::MessageLifecycle;

    #[test]
    fn content_and_attachments_are_visible_without_delivery_diagnostics() {
        let mut attachment = styrene_ipc::types::AttachmentInfo::default();
        attachment.name = "field-notes.txt".into();
        attachment.size = 128;
        attachment.availability = "unavailable".into();
        let message = ChatMessage {
            id: "test-message".into(),
            source: String::new(),
            destination: "peer".into(),
            content: "First line\nSecond line".into(),
            timestamp: 1_788_777_000,
            ingest_sequence: None,
            is_outgoing: true,
            status: "queued".into(),
            lifecycle: MessageLifecycle { attachments: vec![attachment], ..Default::default() },
        };
        let mut dom = VirtualDom::new_with_props(MessageBubble, MessageBubbleProps { message });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("First line\nSecond line"));
        assert!(html.contains("field-notes.txt"));
        assert!(html.contains("Not available on this device"));
        assert!(!html.contains("<dialog"), "diagnostics are not mounted until requested");
        assert!(!html.contains("Delivery evidence"));
        let caption = html.find("<footer").expect("caption outside bubble");
        assert!(html[..caption].contains("</div>"));
        assert!(html[caption..].contains("data-message-time"));
        assert!(html[caption..].contains("aria-haspopup=\"dialog\""));
    }

    #[test]
    fn lifecycle_view_keeps_missing_daemon_fields_explicit() {
        let message = ChatMessage {
            id: "correlation".into(),
            source: String::new(),
            destination: "peer".into(),
            content: "message".into(),
            timestamp: 0,
            ingest_sequence: None,
            is_outgoing: true,
            status: "pending".into(),
            lifecycle: MessageLifecycle::default(),
        };
        assert!(message.lifecycle.requested_method.is_none());
        assert!(message.lifecycle.actual_method.is_none());
        assert!(message.lifecycle.fallback_reason.is_none());
        assert!(message.lifecycle.correlation_id.is_none());
        assert!(message.lifecycle.attempts.is_empty());
        assert!(message.lifecycle.propagation.is_empty());
    }
}

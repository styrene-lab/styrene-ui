use dioxus::prelude::*;
use styrene_ipc::types::{FileDownloadInfo, FileDownloadState};

fn filename(path: &str) -> String {
    let name: String = path
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty() && *part != "." && *part != "..")
        .unwrap_or("download")
        .chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect();
    if name.is_empty() { "download".into() } else { name }
}

fn status(download: &FileDownloadInfo) -> &'static str {
    match download.state {
        FileDownloadState::Pending => "Starting download…",
        FileDownloadState::Receiving => "Downloading…",
        FileDownloadState::Completed if download.integrity_verified => "Ready to save",
        FileDownloadState::Completed => "Verification unavailable",
        FileDownloadState::Saved => "Saved",
        FileDownloadState::Cancelled => "Download cancelled",
        FileDownloadState::Failed => "Download failed",
        _ => "Waiting for download status…",
    }
}

fn percent(value: f32) -> u32 {
    if value.is_finite() { (value.clamp(0.0, 1.0) * 100.0).round() as u32 } else { 0 }
}

#[component]
pub fn DownloadCard(
    download: FileDownloadInfo,
    available: bool,
    action: Option<String>,
    error: Option<String>,
    on_cancel: EventHandler<String>,
    on_save: EventHandler<(String, String)>,
) -> Element {
    let mut choosing = use_signal(|| false);
    let mut dialog_error = use_signal(|| None::<String>);
    let name = filename(&download.native_path);
    let active = !download.state.is_terminal();
    let ready = download.state == FileDownloadState::Completed && download.integrity_verified;
    let busy = choosing() || action.is_some();
    let label = action.as_deref().unwrap_or_else(|| status(&download));
    rsx! {
        section { class: "download-card", aria_label: "File download",
            div { class: "download-summary",
                div { class: "download-description",
                    strong { "{name}" }
                    span { role: "status", "{label}" }
                }
                if active {
                    button { aria_label: "Cancel download", disabled: busy || !available, onclick: { let id = download.download_id.clone(); move |_| on_cancel.call(id.clone()) }, "Cancel" }
                }
                if ready {
                    button { class: "download-save", aria_label: "Save downloaded file", disabled: busy || !available,
                        onclick: {
                            let name = name.clone(); let id = download.download_id.clone();
                            move |_| {
                                choosing.set(true); dialog_error.set(None);
                                let name = name.clone(); let id = id.clone();
                                spawn(async move {
                                    let selected = rfd::AsyncFileDialog::new().set_title("Save download").set_file_name(name).save_file().await;
                                    if let Some(file) = selected {
                                        match file.path().to_str() {
                                            Some(path) => on_save.call((id, path.to_string())),
                                            None => dialog_error.set(Some("This filename cannot be sent to the daemon. Choose another name.".into())),
                                        }
                                    }
                                    choosing.set(false);
                                });
                            }
                        },
                        if choosing() { "Choose location…" } else if action.is_some() { "Saving…" } else { "Save…" }
                    }
                }
            }
            if active {
                div { class: "download-progress",
                    if download.total_bytes > 0 {
                        progress { max: "100", value: "{percent(download.progress)}", aria_label: "Download progress" }
                        span { "{percent(download.progress)}%" }
                    } else { progress { aria_label: "Waiting for transfer" } }
                }
            }
            if ready { p { "Transfer verified. Choose where to save your file." } }
            if let Some(path) = download.saved_path.as_ref() { p { class: "download-saved-path", "Saved to {path}" } }
            if download.state == FileDownloadState::Failed { p { class: "download-error", role: "alert", "The transfer did not complete. Open the file link to try again." } }
            if download.state == FileDownloadState::Completed && !download.integrity_verified { p { class: "download-error", "This file cannot be saved because verification did not complete." } }
            if let Some(message) = error.or(dialog_error()) { p { class: "download-error", role: "alert", "{message}" } }
            details { class: "download-details",
                summary { "Transfer details" }
                dl {
                    dt { "Source" } dd { "{download.host_hash}:{download.native_path}" }
                    dt { "Transfer bytes" } dd { "{download.received_bytes} / {download.total_bytes}" }
                    dt { "SHA-256" } dd { {download.sha256.as_deref().unwrap_or("Not available")} }
                    dt { "Transfer" } dd { "{download.transfer:?}" }
                    dt { "Request" } dd { "{download.correlation_id}" }
                    dt { "Resource" } dd { {download.resource_hash.as_deref().unwrap_or("Not applicable")} }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_requires_verification_and_is_distinct_from_saved() {
        let mut download = FileDownloadInfo::default();
        download.state = FileDownloadState::Completed;
        assert_eq!(status(&download), "Verification unavailable");
        download.integrity_verified = true;
        assert_eq!(status(&download), "Ready to save");
        download.state = FileDownloadState::Saved;
        assert_eq!(status(&download), "Saved");
    }
    #[test]
    fn dialog_name_and_progress_do_not_expose_raw_protocol_values() {
        assert_eq!(filename("/file/nested/hello.txt"), "hello.txt");
        assert_eq!(filename("../../hello.txt"), "hello.txt");
        assert_eq!(percent(f32::NAN), 0);
        assert_eq!(percent(1.2), 100);
        assert_eq!(percent(-1.0), 0);
        assert_eq!(percent(0.42), 42);
    }
}

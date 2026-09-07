# Design

The WebKit viewport controller owns geometry and local date formatting. Dioxus owns message content and dialog visibility. Stable message IDs and a keyed peer container isolate observers between conversations. Observers and the recency timer are removed when the viewport unmounts.

The first loaded page and a newly appended outgoing message anchor to the bottom. Incoming messages and resized content follow only when pinned; otherwise the first visible message remains the anchor. New incoming messages expose Jump to latest. Pagination preserves the visible anchor. No smooth animation is needed.

Under one hour, captions use recency; from one hour onward they show local calendar date and time, including the year when needed. Full local timestamps are available on hover. Future or invalid timestamps never appear as negative recency.

The bubble owns content and a separate attachment area. The caption owns time and a quiet delivery-details button. The native dialog provides modal focus, Escape dismissal and focus return. Retry/cancel stay outside the bubble.

Attachment metadata is visible now, including filename, size, and reported availability. Future image previews and file actions belong in this attachment area, backed by typed content/transfer APIs. The UI must not infer image URLs from filenames or present download/open controls before a usable content handle is available. Failed transfers and progress belong on the attachment tile; protocol evidence remains in the details dialog.

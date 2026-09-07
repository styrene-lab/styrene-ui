# Design

The viewport reports its latest incoming message only while pinned, focused, visible, and outside a modal. A generation-scoped Dioxus listener queues `MarkRead`; the typed broker uses the existing IPC client. A conversation is the current backend read unit: the command marks all incoming messages for that peer, not individual visibility receipts.

The daemon outcome supplies the unread count. Legacy count-only responses trigger an authoritative conversation query. Failure leaves the count intact and exposes a retry control. Stale generations and mismatched peer acknowledgements cannot change it. Message and messaging-operation events refresh conversation summaries so newly arriving unreads and other clients’ read operations become visible. No remote read-receipt protocol is added.

The viewport controller stays alive until unmount, then removes focus/visibility listeners, observers and its timer. Session generation is part of the thread key so reconnect/profile replacement cannot keep an obsolete read observer.

# Desktop message composer

Place a multiline message body before the toolbar. Enter inserts a newline;
Command/Ctrl+Enter sends under the same validation as Send. An empty body disables
Send without a warning. Keep session and validation failures visible.

Place title, delivery method and draft discard under Message options. Preserve
existing typed send and attachment limits. Style the native file input as Attach
files, show removable file chips, and show the attachment persistence note only
when attachments exist. Keep the native picker accessible to keyboard users.

This is desktop presentation work; mobile composer work remains independent.

## Message lifecycle controls

Align compact, styled actions with the outgoing message. Delivered and other
terminal nonretryable messages have no Retry/Cancel controls. Retry requires the
daemon's reported eligibility and a failed, expired or recoverable queued state.
Cancel is offered while queued or sending. The daemon remains authoritative and
can reject an action if transport dispatch or another transition has raced it.

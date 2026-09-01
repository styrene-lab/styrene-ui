# Styrene Wry Patch

This directory vendors the crates.io `wry` 0.55.1 source. The Android WebView
lookup first uses Wry's exact WindowManager match, then falls back when exactly
one activity is registered. Some commodity-device WindowManager proxies do not
preserve the JNI object identity expected by the exact lookup. Cold starts also
wait up to Wry's existing main-pipe timeout for JNI activity registration rather
than panicking when Rust requests the WebView first.

The public Android `try_dispatch` bridge also reports a missing activity or full
main-thread queue instead of panicking or blocking. Native Styrene platform
adapters use this path during activity startup and teardown.

The Android activity template forwards `onConfigurationChanged` to an optional
Rust callback. Styrene uses the callback to request a generation-safe WebView
platform resnapshot after native font-scale and configuration changes.

The activity template also owns the bounded lifetime of Android's dynamic USB
permission receiver. It validates the callback against the requested device
name before forwarding the one-shot result to safe application code.

The same activity template exposes Android's system document picker through a
one-shot URI callback. Product workflow state and document reads remain in
Rust; the activity owns only intent presentation and result forwarding.

Encrypted identity backup sharing uses the same activity template and an
exact-path, non-exported content provider. It writes at most 16 MiB to one
private cache file, presents an `ACTION_SEND` chooser with a read-only URI
grant, and removes the file on presentation failure, return to the activity,
activity destruction, or the next activity creation.

The fallback remains unavailable when multiple activities are registered, so
ambiguous multi-window routing still fails closed. Remove this patch after the
equivalent behavior is available in the pinned upstream Wry release.

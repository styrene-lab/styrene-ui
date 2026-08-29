# Styrene Wry Patch

This directory vendors the crates.io `wry` 0.55.1 source. The Android WebView
lookup first uses Wry's exact WindowManager match, then falls back when exactly
one activity is registered. Some commodity-device WindowManager proxies do not
preserve the JNI object identity expected by the exact lookup.

The fallback remains unavailable when multiple activities are registered, so
ambiguous multi-window routing still fails closed. Remove this patch after the
equivalent behavior is available in the pinned upstream Wry release.

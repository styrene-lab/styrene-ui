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

The fallback remains unavailable when multiple activities are registered, so
ambiguous multi-window routing still fails closed. Remove this patch after the
equivalent behavior is available in the pinned upstream Wry release.

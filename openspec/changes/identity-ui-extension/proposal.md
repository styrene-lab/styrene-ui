# First Identity UI extension

## Intent

Let the independently owned Identity component supply its own management page
inside the mesh application. Use this first extension to test lifecycle behavior
before generalizing a plugin system.

## Scope

One optional, build-time Dioxus extension; one named page; a small registration
record; an injected typed Identity client; host-owned navigation and lifecycle.
The first page shows public identity/capability state and exercises one bounded,
read-only provider operation. Mutating identity operations follow the Identity
owner's reviewed contract and the existing CLI acceptance matrix.

This change does not introduce dynamic libraries, Wasm hosting, downloads, a
marketplace, a UI schema language, arbitrary extension points, or a new repository
hierarchy. Enabling a bundled extension is not installing new executable code.

## Success criteria

- The application operates with the extension absent, disabled, or unavailable.
- The extension renders its own page within the host shell using shared styling.
- The first boundary contains only what that page and its lifecycle need.
- Provider capability failures and stale session responses have tested behavior.
- Disable, re-enable, navigation, restart, and unfinished-operation semantics are
  explicit; no UI teardown silently cancels a backend operation or deletes identity.
- The dogfood results identify which general plugin decisions can remain deferred.

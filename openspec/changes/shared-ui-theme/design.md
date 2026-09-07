# Design

Store tweakcn-compatible `:root` and `.dark` declarations in `assets/theme/preset.css`. Shared `tokens.css` maps semantic roles to existing component vocabulary. Both Dioxus roots embed the same CSS at compile time; no renderer dependency or theme crate is needed. OS appearance selects the palette. Layout, target sizes, and safe areas stay in each shell stylesheet.

Orange is `primary`; tweakcn `accent` is a quiet highlight surface. Success, warning, failure, and unknown remain Styrene semantic extensions, independent of user theme chart colors. A future validated export replaces the preset; accepting arbitrary CSS at runtime is outside this change.

The published cmtr8h1vu000204i6gaao9ld2 theme supersedes the starting preset. Both
OKLCH palettes and common values are preserved, with a registry snapshot and a
token fidelity test. Small light-mode action text uses a separate contrast-tested
ink instead of modifying the canonical primary. Font bytes and OFL licenses are
bundled once in the shared CSS source so offline nodes need no font provider.

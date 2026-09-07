# Design

Store tweakcn-compatible `:root` and `.dark` declarations in `assets/theme/preset.css`. Shared `tokens.css` maps semantic roles to existing component vocabulary. Both Dioxus roots embed the same CSS at compile time; no renderer dependency or theme crate is needed. OS appearance selects the palette. Layout, target sizes, and safe areas stay in each shell stylesheet.

Orange is `primary`; tweakcn `accent` is a quiet highlight surface. Success, warning, failure, and unknown remain Styrene semantic extensions, independent of user theme chart colors. A future validated export replaces the preset; accepting arbitrary CSS at runtime is outside this change.

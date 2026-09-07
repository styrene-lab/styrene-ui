# Shared Field Kit theme

`preset.css` preserves the published [Styrene Field Kit](https://tweakcn.com/themes/cmtr8h1vu000204i6gaao9ld2) theme, retrieved on 2026-09-07.
`published.json` retains the registry export from `https://tweakcn.com/r/themes/cmtr8h1vu000204i6gaao9ld2`. Both light (`:root`) and
dark (`.dark`) must be retained when replacing it with a reviewed export.
`tokens.css` owns Styrene aliases and operational status roles. Chart colors
are decorative and do not redefine success, warning, failure, or unknown.

After replacing the preset, run from the repository root:

```sh
python3 assets/theme/prepare.py
python3 assets/theme/prepare.py --check
cargo test -p styrene-ui-app --test theme
```

The checked-in `runtime.css` adds automatic system dark appearance selection.
Both Dioxus shells embed it and `tokens.css` before their local stylesheet.
Desktop keeps its navigation rail, panes, and table density. Mobile keeps its
navigation, safe areas, reflow, and platform touch targets. No runtime theme
import or editor is included.

Actions use `primary` and its paired foreground: orange in light mode and green in dark mode.
Small light-mode action text uses the contrast-tested `action-ink` adapter; the canonical primary is unchanged. `accent` is a quiet selected
surface. Shape uses `radius`; typography uses `font-sans` and `font-mono`.
Shared changes must be checked against both palettes and both shell builds.
The contrast test supports opaque hex and numeric OKLCH colors, checks sRGB fallback contrast, and verifies conversion against reference colors. A second gate compares every exported token with the published snapshot.

`fonts.css` bundles the selected fonts without network requests. See `fonts.md` for provenance and licenses. The shared adapter applies the exported letter spacing to both shells.

## Pending published refresh

On 2026-09-07 Wilson supplied `#68b145` for the dark primary/selected
background. The registry endpoint still returned its previous snapshot after
a cache-busting request. `tokens.css` applies that exact value to dark `primary`
and `sidebar-primary`; the retrieved export remains intact. Remove this
explicit override when the published snapshot carries the update.

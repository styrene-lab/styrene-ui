# Shared Field Kit theme

`preset.css` is the portable tweakcn starting preset. Both light (`:root`) and
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

Orange uses `primary` and its paired foreground. `accent` is a quiet selected
surface. Shape uses `radius`; typography uses `font-sans` and `font-mono`.
Shared changes must be checked against both palettes and both shell builds.
The contrast test currently covers the shipped hex palette; extend the color
parser when adopting an OKLCH export, rather than skipping the contrast gate.

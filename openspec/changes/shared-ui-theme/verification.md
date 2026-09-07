# Verification — 2026-09-07

Implementation commit: `6deeaae`. Backend pin: `15e80f03bcd4cbed55e2e748173eb39b4c0c3c37`.
The independent pre-existing mobile work remains uncommitted and was preserved;
shared-shell tests and the simulator build include that working tree. Theme changes
were staged independently from it.

- Desktop: 165 passed, 4 intentionally ignored integration tests.
- Shared mobile presentation: 95 passed across dispatch, drafts, shell, and theme.
- `cargo clippy -p styrene-dx -p styrene-ui-app --all-targets -- -D warnings`: passed.
- Changed Rust files: rustfmt check passed.
- `python3 assets/theme/prepare.py --check`: passed.
- `just mac-app`: passed; run `macos-app-iocmbqzi`.
- `just ios-build`: passed for `aarch64-apple-ios-sim`; run `apple-mobile-nb2leca3`.
- Android/Linux and physical Apple runtime checks: not executed here. Shared theme
  changes require Nucleus validation before a shared platform baseline advances.

The first shared-shell compile reused stale state-crate output. Invalidating that
crate's source timestamp rebuilt it successfully; no state source edits were made.
The palette contrast test found the original light primary on canvas at 3.86:1.
The app preset now uses `#ac3c18` for small action text; the original Documents
palette remains Wilson's editable starting point. All tested text pairs pass 4.5:1.

## Runtime evidence

Evidence lives in the outer checkout at `lab/runs/shared-ui-theme/`, including
source fingerprints, build output, process identity, binary SHA-256, test logs,
and capture hashes. The live daemon was preserved (PID 53894). Desktop PID 75157
started at 08:27:54 EDT from the newly built `StyreneDx.app` under the canonical UI
clone. The saved Browse route was restored despite the messages launch hint.

| Input | Observation | Decision |
|---|---|---|
| Shared preset and desktop aliases; same Browse composition | Dark capture shows orange selected rail item, square controls, condensed headings, muted green-black surfaces; session ready | Visual foundation accepted |
| OS light appearance, same process | Window 3087 capture shows cream surfaces, dark text, retained rail and directory geometry, rendered Pi fixture page | Automatic light mode verified; restored OS dark appearance |

An initial light desktop capture was obscured by another application and discarded.
The replacement captures only the Styrene window. No iOS runtime screenshot or
Android runtime evidence is claimed from a successful simulator package build.

## Published theme adoption — 2026-09-07

Source: https://tweakcn.com/themes/cmtr8h1vu000204i6gaao9ld2
Registry endpoint: https://tweakcn.com/r/themes/cmtr8h1vu000204i6gaao9ld2
`assets/theme/published.json` retains the retrieved export. Every common and
mode-specific token is checked against `preset.css`; fonts are embedded with
licenses and SHA-256 provenance. OKLCH contrast conversion passes black, white,
and sRGB primary reference vectors. Published primary colors remain intact.

- Shared mobile presentation: 97 tests passed, including 3 theme tests.
- Desktop: 165 passed; 4 integration tests remain intentionally ignored.
- Warning-denied Clippy passed after naming color-conversion test variables.
- Native Mac build: `macos-app-jpdfjbxz`, passed.
- iOS simulator package: `apple-mobile-l0620nik`, passed.
- Desktop capture: window 3213, Overview, Live session ready; green primary
  selection and Electrolize rendered in the existing desktop composition.
- Source state, artifact and capture hashes, process identity and logs: outer
  `lab/runs/published-ui-theme/`. The build includes preserved mobile WIP.
- Android/Linux, physical Apple runtime and mobile runtime visuals were not
  executed. Dark desktop rendering was inspected; both palettes passed the
  automated contrast gate.

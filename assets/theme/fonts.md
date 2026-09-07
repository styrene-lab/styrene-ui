# Offline fonts

Bundled for the published Styrene Field Kit theme on 2026-09-07.
The CSS contains font bytes directly so both Dioxus renderers can embed the
same source without platform-specific asset URLs or external requests.

| Family | Weight | Bytes | SHA-256 of font bytes |
|---|---|---|---|
| Electrolize | 400 | 55712 | `aa81ab9015fc0190bb5e68c50290565c7a4a723ddf32b88774aaff05cdd66bca` |
| JetBrains Mono | 100 800 | 187208 | `48715a42ec242c21e9f02692891e147d022299a52e48d5e413e1a942193ffeda` |
| Merriweather | 400 | 585976 | `e029d3b71e5bae3877c73f4f2b15af28db16593d49b0aed2a328d37ff7075846` |

Electrolize and variable JetBrains Mono were retrieved from the Google Fonts
repository (`ofl/electrolize/Electrolize-Regular.ttf` and
`ofl/jetbrainsmono/JetBrainsMono[wght].ttf`). Merriweather regular was retrieved
from Google Fonts CSS API, family Merriweather, weight 400. The retained font
licenses are in `licenses/`. Other weights for Electrolize and Merriweather use
renderer synthesis; unsupported glyphs use the exported system fallbacks.

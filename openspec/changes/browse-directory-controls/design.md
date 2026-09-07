# Design

Keep the directory compact: search, collection switch, expandable filters, count,
local pages and bounded results. Favorite actions are separate sibling buttons,
never nested inside navigation buttons. Save up to 256 public address/name pairs
in a desktop-local JSON file under XDG_CONFIG_HOME/styrene-ui (or ~/.config).
No private identity, daemon cache or remote capability is inferred from favorites.
The parser bounds stored input; atomic replacement preserves existing favorites
on write failure. A minute tick updates age filters. Existing backend-session
visit evidence remains authoritative and is not persisted as reachability.

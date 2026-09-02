# Extraction Provenance

The repository began as a history-preserving extraction of
`crates/apps/styrene-dx` from `https://github.com/styrene-lab/styrene-rs.git`.

- Source branch: `feat/mobile-platform-hosts`
- Source revision at extraction: `4bcbc68f4424cc69a775a6508c5c069386ff776b`
- Extracted path: `crates/apps/styrene-dx`
- Initial extracted revision: `a769d2c`
- Destination: `https://github.com/styrene-lab/styrene-ui.git`
- Desktop synchronization revision: `d8c0569fc30dd49280a3246703a31c84bb646bf3`
- Tested backend contract revision: `d8c0569fc30dd49280a3246703a31c84bb646bf3`

The extraction ran from an isolated clone with:

```bash
git subtree split --prefix=crates/apps/styrene-dx -b styrene-ui-main
git push styrene-ui styrene-ui-main:main
```

The source checkout and its history were not rewritten.

Before the authority switch, the extracted desktop tree was synchronized with
the source tree at the desktop synchronization revision above. All desktop
backend dependencies are Git dependencies pinned to that same full revision;
the workspace does not require a sibling `styrene-rs` checkout.

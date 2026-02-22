# Quickstart: Memory Optimization

**Feature**: 009-memory-optimization
**Date**: 2026-02-22

## Overview

Optimize peak memory consumption in clio by eliminating redundant image buffer clones, sequencing clipboard reads, adding early size rejection, and caching decoded thumbnails.

## Files to Modify

| File | Change |
|------|--------|
| `src/models/entry.rs` | Change `encode_rgba_to_png` to accept `Vec<u8>` by ownership; update `from_image` to pass owned buffer |
| `src/cli/watch.rs` | Restructure poll loop for sequential CLIPBOARD/PRIMARY processing; add early RGBA size check before encoding |
| `src/ui/window.rs` | Add bounded thumbnail texture cache keyed by content hash |

## No New Dependencies

All changes use existing crate APIs. No new dependencies required.

## Build & Test

```bash
# Tests (no GTK4 required)
cargo test --no-default-features

# Full build + clippy (requires GTK4 via guix)
guix shell -m manifest.scm -- cargo clippy
guix shell -m manifest.scm -- cargo test
```

## Verification

1. `cargo test --no-default-features` — all existing tests pass
2. `guix shell -m manifest.scm -- cargo clippy` — no warnings
3. Manual: copy large image with `clio watch` running, observe RSS stays lower

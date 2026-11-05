# Verification

`palette.rs` is a Verus model of the result selection in `src/search.rs`.

| Model | Runtime | Proves |
|---|---|---|
| `take_ranked` | `search::take_ranked` | `limit` or fewer, each index one time, each index is a match, rank order |
| `dedupe_capped` | `search::parse_recent` | `max` or fewer, each id one time, each id is from the input |

A `u64` replaces the id string. The model uses only equality.

Verify it:

```sh
verus verification/palette.rs
```

The property tests in `src/search.rs` check the same postconditions on the
runtime code. They also check that no match is lost. The `verus` CI job
runs the model with a pinned Verus release.

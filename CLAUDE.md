# CLAUDE.md - autumn-command-palette-plugin

A ⌘K command palette widget pack for Autumn (autumn#3416).

## Layout

| Path | Content |
|---|---|
| `src/command.rs` | `Command`, `CommandId`, `CommandRegistry` |
| `src/hotkey.rs` | `Hotkey` spec parser |
| `src/search.rs` | ranking and recent ids (follows `verification/palette.rs`) |
| `src/render.rs` | all markup |
| `src/palette.rs` | `CommandPalette`, `Labels`, `pack()`, `Plugin` impl |
| `src/route.rs` | `GET /_palette` |
| `src/admin.rs` | `admin` feature |
| `src/command-palette.{css,js}` | pack assets |
| `tests/integration.rs` | `TestApp` tests |
| `tests/browser.rs` | Chromium tests (`system-tests`) |
| `verification/palette.rs` | Verus model |

## Rules

- Obey the autumn widget contract (`docs/guide/widget-packs.md` in autumn).
  Classes are `autumn-command-palette-*` or core classes. CSS uses public
  tokens only. JS has no arrow-key code and no inline handlers.
- If you change `take_ranked` or `parse_recent`, change the Verus model too.
- Comments and docs use ASD-STE100: short sentences, active voice.
- Do not cite review rounds or findings in comments. An issue number is OK.

## Commands

- Lint: `cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings`
- Test: `cargo test --all-features`
- Browser: `cargo test --features system-tests --test browser -- --include-ignored`
- Coverage: `cargo llvm-cov --all-features --fail-under-lines 85`
- Verus: `verus verification/palette.rs`

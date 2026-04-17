# yoz — Terminal Text Editor

## Project Overview
yoz is a terminal-based text editor written in Rust using `crossterm` for terminal manipulation. It is inspired by `nano` — simple, modal-free, and easy to use.

## Architecture

### Modules
- **main.rs** — CLI entry point, parses args and launches editor
- **editor.rs** — Core editor logic: event loop, key/mouse handling, undo/redo, search/replace, clipboard
- **buffer.rs** — Text buffer (Vec<String> lines), file I/O, character-level operations
- **terminal.rs** — Thin crossterm wrapper (raw mode, alternate screen, mouse capture, synchronized output)
- **ui.rs** — Rendering: text area with line numbers, status bar, message bar, scrollbar, help overlay
- **encoding.rs** — Multi-encoding support: detect (BOM + chardetng), decode, encode (encoding_rs)
- **selection.rs** — Selection model with anchor+cursor range
- **history.rs** — Undo/redo stack with action grouping

### Key Design Decisions
- **No modal editing** — always in insert mode (like nano, not vim)
- **crossterm only** — no ratatui; full control over every cell for editor-specific rendering
- **Synchronized output** — BeginSynchronizedUpdate/EndSynchronizedUpdate to prevent flicker
- **Event-driven rendering** — only redraws when an event occurs or status message expires
- **Adaptive scroll acceleration** — scroll speed increases with consecutive scroll events within 80ms
- **Character-indexed operations** — all cursor/selection positions are char-indexed (not byte-indexed) for correct UTF-8/multi-byte handling

### Encodings Supported
UTF-8, UTF-8 BOM, UTF-16 LE, UTF-16 BE, Windows-1251, Windows-1252, CP866

### Dependencies
- `crossterm` — terminal control
- `encoding_rs` — encoding/decoding
- `chardetng` — encoding auto-detection
- `arboard` — system clipboard

## Build & Run
```bash
cargo build --release
cargo install --path .
yoz <file>
```

## Conventions
- Status messages auto-clear after 5 seconds
- Line endings (LF/CRLF) are preserved from original file
- Files are saved in their original encoding
- Undo groups multi-character operations (paste, word delete, selection delete)

# PDFEdit.Rust
High level PDF editor written in rust

## Features

- Open and view multi-page PDF documents
- **Smooth continuous scrolling** – all pages are laid out vertically in a single scrollable canvas. Scroll freely across page boundaries with the mouse wheel or trackpad. Only visible pages are rendered for performance.
- Annotation tools: Highlight, TextBox, Freehand Draw, Rectangle, Arrow, Underline, Strikethrough, Comment, Signature, Eraser
- Zoom in/out and fit-to-window
- Undo / redo annotation history
- OCR via Tesseract
- Export to images (PNG/JPEG/TIFF) or Word (DOCX)
- Save and reload annotations (JSON sidecar)

## Navigation

| Action | Method |
|--------|--------|
| Scroll through pages | Mouse wheel / trackpad scroll |
| Jump to a page | Click the page entry in the left "Pages" sidebar |
| Previous / next page | ◀ / ▶ buttons in the toolbar, or `←` / `→` arrow keys |
| Keyboard page jump | `PageUp` / `PageDown` keys |
| Zoom | 🔍− / 🔍+ buttons |
| Fit to window | ⊙ button in the toolbar |

## Toolbar icons

Toolbar icons are [Lucide](https://lucide.dev/) SVGs stored in `assets/icons/lucide/`.
They are **embedded in the binary** at compile time via `include_bytes!` – no files need to be shipped alongside the executable.

### Adding a new toolbar icon

1. Download the `.svg` from <https://lucide.dev/icons/> and save it to `assets/icons/lucide/<name>.svg`.
2. In `src/ui/toolbar.rs`, add a constant:
   ```rust
   const ICON_MY_ICON: &[u8] = include_bytes!("../../assets/icons/lucide/<name>.svg");
   ```
3. Call `svg_icon_button` (or `svg_tool_button` for toggle tools):
   ```rust
   svg_icon_button(ui, &mut state.icon_cache, "icon_my_icon", ICON_MY_ICON, "Tooltip text")
   ```

The icon cache (`src/ui/icons.rs` → `IconCache`) rasterises each SVG once per unique *(key, pixel-size)* pair and caches the resulting `egui::TextureHandle` for the lifetime of the app.

## Requirements

- Rust (stable)
- `pdftoppm` (part of `poppler-utils`)
- `tesseract` (for OCR)
- `libleptonica-dev` / `libtesseract-dev` (build dependencies)

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

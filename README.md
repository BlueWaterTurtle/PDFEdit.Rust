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

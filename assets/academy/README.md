# Academy course artwork

Course insignia shown on Home progress cards and the Academy course
picker. Both are rendered from their vector sources at 2x for crisp
display; both use transparency (RGBA).

- `javascript-logo.png` — the classic JavaScript yellow-square mark
  (yellow #F7DF1E field with dark "JS" letterforms). Source: the
  widely used community vector (Wikimedia Commons "Unofficial
  JavaScript logo_2.svg", public-domain style availability; "Java" and
  "JavaScript" are Oracle trademarks — we use the mark to identify the
  language, not to imply endorsement).
- `rust-logo.png` — the official Rust Foundation gear logo (black
  variant). Source: rust-lang/rust-artwork `rust-logo-blk.svg` via
  Wikimedia Commons. Rust Foundation logo policy permits use of the
  unmodified logo to identify the Rust language; we do not modify it
  beyond rasterization and do not imply endorsement.

If either mark needs replacing (policy change, official request), swap
the PNG here and the app picks it up on next build — the Rust binary
embeds these via include_bytes!.

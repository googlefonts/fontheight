# `harfshapedfa`

Some glue and utilities to ease working with `harfrust` and `skrifa`, continuing the tradition of confusing font-related crate names.

## What exciting features can you offer?

<!-- TODO: make these docs.rs links -->
- `ShapingMeta`, to make creating & re-using [shaping plans](https://harfbuzz.github.io/shaping-plans-and-caching.html) easier
- `Location`, a library-agnostic variable font location specifier, mapping axis names to values. Can be validated against a font
- Conversion functions between script & direction for `harfrust` (currently private API), and between ISO 15924 script tags and OpenType feature script tags

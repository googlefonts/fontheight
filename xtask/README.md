# fontheight cargo xtasks

For a primer on the Cargo xtask pattern: see [this repo](https://github.com/matklad/cargo-xtask).

TL;DR `cargo xtask <task_name>`.

## `slwl`: code generation for `static-lang-word-lists`

This task discovers word lists within `static-lang-word-lists/data` & reads their metadata files,.
From this, it writes `static-lang-word-lists/chicken.rs`, a list of all the paths to word list data files, which is read by `static-lang-word-lists`' build script to see what's necessary to compress.
It also writes `static-lang-word-lists/src/declarations.rs`, which contains the definitions of the word list constants and `LOOKUP_TABLE`.

The task will also print the Cargo.toml feature declarations, as each word list will be gated behind feature gates, based on its source (e.g. `diffenator`), script (e.g. `script-latn`), and language (e.g. `lang-en`).
It does not update `static-lang-word-lists/Cargo.toml` itself.

### Usage

```
Usage: cargo xtask slwl [OPTIONS]

Options:
  -q, --no-emit-features    Don't print Cargo.toml feature declarations
```

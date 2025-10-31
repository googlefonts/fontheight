# `harfshapedfa`

Some glue and utilities to ease working with [`harfrust`](https://docs.rs/harfrust) and [`skrifa`](https://docs.rs/skrifa), continuing the tradition of confusing font-related crate names.

> *This crate is not affiliated with `harfrust` or `skrifa`*

## What exciting features can you offer?

- [`ShapingMeta`](https://docs.rs/harfshapedfa/latest/harfshapedfa/struct.ShapingMeta.html), to make creating & re-using [shaping plans](https://harfbuzz.github.io/shaping-plans-and-caching.html) easier
- [`Location`](https://docs.rs/harfshapedfa/latest/harfshapedfa/struct.Location.html), a library-agnostic variable font location specifier, mapping axis names to values. Can be validated against a font
- Conversion functions between script, language, and direction (to `harfrust` or OpenType types)

The hope would be to see this crate eventually deprecated as the functionality/utilities provided here move into sensible locations in the major font libraries, like `harfrust`/`skrifa`/fontations.

## Usage

This crate basically expects you to already be using [`harfrust`](https://docs.rs/harfrust) and [`skrifa`](https://docs.rs/skrifa) - if you're not this probably isn't the crate for you.

Your Cargo.toml will probably look something like this:

```toml
[dependencies]
harfrust = "=0.3.2"
skrifa = "=0.37.0"
harfshapedfa = "0.1"
```

Note: `harfshapedfa` pins against very specific version of `skrifa` and `harfrust`, as both projects have seen breaking changes on minor releases, usually by bumping `read-fonts`.
By keeping everything in lockstep, we avoid duplicate dependencies and incompatible types due to different versions of the same types being used.

### Pens

`harfshapedfa` also exports pens optionally if you enable the crate's `pens` feature:

```toml
[dependencies]
harfrust = "=0.3.2"
skrifa = "=0.37.0"
harfshapedfa = { version = "0.1", features = ["pens"] }
```

This provides some pens and re-exports some [`kurbo`](https://docs.rs/kurbo/latest/kurbo/) types that our API exposes, so if you have a very simple use case you probably won't need to explicitly pull in `kurbo`.

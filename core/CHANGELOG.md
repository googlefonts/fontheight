# `fontheight` library crate changelog

## v0.2.0

**Breaking change**: the `Location` type and several errors now reside in [`harfshapedfa`](../harfshapedfa/README.md) instead of this crate. Error types of some functions have changed.

### Removed

All now live in [`harfshapedfa`](../harfshapedfa/README.md).
Other changes have been made to the `Location` API/behaviour (see [`harfshapedfa/CHANGELOG.md`](../harfshapedfa/CHANGELOG.md).

- `Location`
- `ShapingPlanError`
- `HarfRustUnknownLanguageError`
- `MismatchedAxesError`
- `InvalidTagError`

### Added

- `WordListShapingPlanError`

## v0.1.8 - 2025/10/27

### Changes

- Sort exemplars lexicographically if their vertical extents match (improves determinism)
- Allow upgrading [`static-lang-word-lists`] to v0.4.0 (the breaking change is not breaking for fontheight, so both versions can be supported)
- Upgrade [`harfrust`]

## v0.1.7 - 2025/09/17

- Allow upgrading [`static-lang-word-lists`] to v0.3.0 (the breaking change is not breaking for fontheight, so both versions can be supported)

## v0.1.6 - 2025/09/15

- Upgrade [`skrifa`] & [`harfrust`]

## v0.1.5 - 2025/09/10

### Internals

- Upgrade [`static-lang-word-lists`] to v0.2.2

## v0.1.4 - 2025/09/10

### Internals

- Upgrade [`static-lang-word-lists`] to v0.2.1

## v0.1.3 - 2025/09/09

### Internals

- Upgrade [`static-lang-word-lists`] to v0.2.0

## v0.1.2 - 2025/08/26

### Added

- `WordExtremes::{lowest,highest,lower,higher}`
- `VerticalExtremes::{new,merge}`

## v0.1.1 - 2025/08/21

- Bump internal dependencies

## v0.1.0 - 2025/08/21

Initial release!

[`static-lang-word-lists`]: ../static-lang-word-lists/CHANGELOG.md
[`skrifa`]: https://lib.rs/crates/skrifa
[`harfrust`]: https://lib.rs/crates/harfrust

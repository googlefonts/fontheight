# `harfshapedfa` library crate changelog

## v0.1.0 - 2025/10/31

Changes documented here are for items that were migrated from `fontheight`, not for any completely new features (e.g. `ShapingMeta`, conversion utilities, and pens).

### Changes

- `Location` is now insertion-order preserving
- `Location` now panics upon injesting any `NaN` values

### Added

- `Location::from_skrifa`
- `Location::to_skrifa`
- `Location::to_harfrust`
- `Location::sort_axes`
- `Location` now implements, `PartialEq`, `Eq`, and `PartialOrd`

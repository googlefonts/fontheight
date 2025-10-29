# `static-lang-word-lists` changelog

## v0.4.1 - 2025/10/29

### Changes

- Added script metadata to all LibreOffice word lists

## v0.4.0 - 2025/10/27

**Breaking changes**: not all word lists are included by default (only diffenator word lists). `LOOKUP_TABLE` has been removed.

### Added

- Feature flags, for any source, script, or language of word list. See [the docs](https://docs.rs/static-lang-word-lists/v0.4.0#feature-flags) for more.
- LibreOffice dictionaries
- `ALL_WORD_LISTS`, a slice with all enabled word lists included

### Changes

- Accessing metadata of static word lists is faster, no longer requiring deserialisation
- Build script should compile & run faster (it's doing less now)

### Removed

- `LOOKUP_TABLE`, use `ALL_WORD_LISTS` instead

## v0.3.1 - 2025/09/23

### Fixes

- Fix crate compile error on GitHub Actions when being used as a dependency

## v0.3.0 - 2025/09/17

**Breaking change**: renamed AOSP word lists

### Changes

- Improved quality of AOSP word lists (filter word lists by script, done [here](https://github.com/googlefonts/aosp-test-texts/pull/10))
- Added script metadata to AOSP word lists
- Improved name of AOSP word lists (including script)
- Compress the word lists for debug builds less to improve compile times

## v0.2.2 - 2025/09/10

### Fixes

- Don't break [docs.rs] builds for crates depending on `static-lang-word-lists`

## v0.2.1 - 2025/09/10

### Fixes

- Get [docs.rs] to generate documentation (successfully this time)

## v0.2.0 - 2025/09/09

**Breaking change**: type of all static word list constants has changed

### Changes

- Constants are no longer `LazyLock<WordList>`, instead `WordList`
- Word list metadata is now deserialised separately to the word list decompression

### Fixes

- Try and get [docs.rs] to generate documentation

## v0.1.0 - 2025/08/21

Initial release!

[docs.rs]: https://docs.rs/static-lang-word-lists

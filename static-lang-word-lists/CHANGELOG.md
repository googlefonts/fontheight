# `static-lang-word-lists` changelog

## v0.3.0 - 2025/09/17

**Breaking change**: renamed AOSP word lists

### Changes

- Improved quality of AOSP word lists (filter word lists by script, done [here](https://github.com/googlefonts/aosp-test-texts/pull/10))
- Added script metadata to AOSP word lists
- Improved name of AOSP word lists (including script)

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

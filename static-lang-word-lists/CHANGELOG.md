# `static-lang-word-lists` changelog

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

//! # `static-lang-word-lists`
//!
//! A collection of word lists for various scripts, compressed at build time
//! with [Brotli](https://brotli.org/), baked into your compiled binary, and
//! decompressed lazily at run time.
//!
//! Word lists are compressed less when building the "debug" profile to speed up
//! build times.
//! Any other profile will use maximum compression.
//!
//! ## Accessing word lists
//!
//! If there's a specific word list you're after, you can refer to its `static`
//! by name.
//! The crate also provides a [`LOOKUP_TABLE`] which maps word list names to
//! their `static`.
//!
//! Word lists are decompressed when you call [`WordList::iter`].
//!
//! ## Feature flags
//!
//! This crate has a plethora of feature flags to help you reduce build times by
//! only compressing the word lists you plan to use.
//!
//! There are three categories of feature flag you can use to choose your word
//! list:
//! - Source name (e.g. `diffenator`), enables all the word lists from a
//!   particular source
//! - Script code (e.g. `script-latn`), enables all the word lists for the [ISO 15924](https://en.wikipedia.org/wiki/ISO_15924)
//!   script (lowercase)
//! - Language code (e.g. `lang-en`), enables all the word lists for the [ISO 639-1](https://en.wikipedia.org/wiki/ISO_639-1)
//!   language
//!
//! You can, of course, mix & match at will. Thanks to the magic of Cargo's
//! [feature unification](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification),
//! any word lists needed by other dependencies will still be pulled in &
//! compiled even if you don't request them in your own crate.
//!
//! If there are no word lists for a script/language, there won't be a feature
//! flag.
//!
//! **By default, only the diffenator word lists are enabled**.
//!
//! ## Creating your own word lists
//!
//! - In memory words: [`WordList::define`]
//! - Word list file (with sidecar metadata): [`WordList::load`]
//! - Word list file (no metadata): [`WordList::load_without_metadata`]
//!
//! ## How this crate works (⚠️disclaimer⚠️)
//!
//! A build script for this crate downloads a zipball of the GitHub repo for
//! this project at build time in order to get the word lists.
//! It is not possible for us to include these in the crate hosted on crates.io
//! as the crate would immediately exceed the size limit.
//!
//! By using a the repository as a path or git dependency you can avoid the
//! download by setting the environment variable `STATIC_LANG_WORD_LISTS_LOCAL`.
//! Otherwise, you're welcome to audit the [build script](https://github.com/googlefonts/fontheight/blob/main/static-lang-word-lists/build.rs).

mod word_lists;

pub(crate) use word_lists::WordListMetadata;
#[cfg(feature = "rayon")]
pub use word_lists::rayon::ParWordListIter;
pub use word_lists::{WordList, WordListError, WordListIter};

use crate::word_lists::{Word, WordSource};

fn newline_delimited_words(input: impl AsRef<str>) -> WordSource {
    input
        .as_ref()
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .map(Word::from)
        .collect()
}

macro_rules! word_list {
    (
        ident: $ident:ident,
        metadata: $metadata:expr,
        bytes: $bytes:expr $(,)?
    ) => {
        /// The
        #[doc = ::std::stringify!($ident)]
        /// word list.
        ///
        /// Compiled into the binary compressed with Brotli, decompressed at
        /// runtime.
        pub static $ident: $crate::WordList = $crate::WordList::new_lazy(
            $metadata,
            ::std::sync::LazyLock::new(|| {
                let mut brotli_bytes: &[u8] = $bytes;
                let mut buf =
                    ::std::vec::Vec::with_capacity(brotli_bytes.len());
                ::brotli_decompressor::BrotliDecompress(
                    &mut brotli_bytes,
                    &mut buf,
                )
                .unwrap_or_else(|err| {
                    ::std::panic!(
                        "failed to decode {}: {err}",
                        ::std::stringify!($ident),
                    );
                });
                let raw_words =
                    // SAFETY: UTF-8 validity is checked by the build script
                    unsafe { ::std::string::String::from_utf8_unchecked(buf) };
                ::log::debug!("loaded words for {}", ::std::stringify!($ident));
                $crate::newline_delimited_words(raw_words)
            }),
        );
    };
}

// Module declaration has to be below macro definition to be able to use it.
// rustfmt::skip applies to the contents of the module, because rustfmt
// traverses modules, not files
#[rustfmt::skip]
mod declarations;
pub use declarations::*;

// TODO: a custom README would be better, talk more about usage than
//       implementation
#![doc = include_str!("../README.md")]

mod word_lists;

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

macro_rules! wordlist {
    (ident: $ident:ident, metadata: $metadata:expr, bytes: $bytes:expr $(,)?) => {
        /// The
        #[doc = ::std::stringify!($ident)]
        /// word list.
        ///
        /// Compiled into the binary compressed with Brotli, decompressed at
        /// runtime.
        pub static $ident: $crate::WordList = $crate::WordList::new_lazy(
            // Note: validity of TOML file was not validated during build,
            // so we must check here
            ::std::sync::LazyLock::new(|| {
                let ret = ::toml::from_str($metadata).unwrap_or_else(|err| {
                    ::std::panic!("failed to deserialize metadata: {err}");
                });
                ::log::debug!(
                    "loaded metadata for {}",
                    ::std::stringify!($ident),
                );
                ret
            }),
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

include!(concat!(env!("OUT_DIR"), "/word_list_codegen.rs"));
include!(concat!(env!("OUT_DIR"), "/map_codegen.rs"));

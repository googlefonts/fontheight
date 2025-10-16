// TODO: a custom README would be better, talk more about usage than
//       implementation
#![doc = include_str!("../README.md")]

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

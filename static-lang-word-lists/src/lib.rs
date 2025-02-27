use std::sync::LazyLock;

pub use fontheight_core::WordList;

pub type LazyWordList = LazyLock<WordList>;

fn newline_delimited_words(input: impl AsRef<str>) -> Vec<String> {
    input
        .as_ref()
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .map(String::from) // FIXME: could be Box<str>
        .collect()
}

macro_rules! wordlist {
    (ident: $ident:ident,name: $name:ident,bytes: $bytes:expr $(,)?) => {
        pub static $ident: $crate::LazyWordList =
            ::std::sync::LazyLock::new(|| {
                static NAME: &str = ::std::stringify!($name);
                let mut brotli_bytes: &[u8] = $bytes;
                let mut buf =
                    ::std::vec::Vec::with_capacity(brotli_bytes.len());
                ::brotli_decompressor::BrotliDecompress(
                    &mut brotli_bytes,
                    &mut buf,
                )
                .unwrap_or_else(|err| panic!("failed to decode {NAME}: {err}"));
                // SAFETY: UTF-8 validity is checked by the build script
                let raw_words = unsafe { String::from_utf8_unchecked(buf) };
                ::log::debug!("loaded {NAME}");
                ::fontheight_core::WordList::new(
                    NAME.to_owned(),
                    $crate::newline_delimited_words(raw_words),
                )
            });
    };
}

include!(concat!(env!("OUT_DIR"), "/word_list_codegen.rs"));
include!(concat!(env!("OUT_DIR"), "/map_codegen.rs"));

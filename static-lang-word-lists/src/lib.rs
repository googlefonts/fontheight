pub use fontheight_core::WordList;

fn newline_delimited_words(input: impl AsRef<str>) -> Vec<String> {
    input
        .as_ref()
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .map(String::from) // FIXME: could be Box<str>
        .collect()
}

macro_rules! wordlist {
    (name: $name:ident, bytes: $bytes:expr $(,)?) => {
        ::paste::paste! {
            pub static [<$name:snake:upper>]: ::std::sync::LazyLock<::fontheight_core::WordList> =
                ::std::sync::LazyLock::new(|| {
                    static NAME: &str = ::std::stringify!($name);
                    let mut brotli_bytes: &[u8] = $bytes;
                    let mut buf = ::std::vec::Vec::with_capacity(brotli_bytes.len());
                    ::brotli_decompressor::BrotliDecompress(&mut brotli_bytes, &mut buf).unwrap_or_else(|err| panic!("failed to decode {NAME}: {err}"));
                    let raw_words = String::from_utf8(buf)
                        .unwrap_or_else(|_| panic!("{NAME} didn't decode to UTF-8"));
                    ::log::debug!("loaded {NAME}");
                    ::fontheight_core::WordList::new(
                        NAME.to_owned(),
                        $crate::newline_delimited_words(raw_words),
                    )
                });
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

fn newline_delimited_words(input: impl AsRef<str>) -> Vec<String> {
    input
        .as_ref()
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .map(String::from)
        .collect()
}

macro_rules! wordlist {
    (name: $name:ident, path: $path:literal $(,)?) => {
        ::paste::paste! {
            pub static [<$name:snake:upper>]: ::std::sync::LazyLock<::fontheight_core::WordList> =
                ::std::sync::LazyLock::new(|| {
                    ::log::debug!("loaded {}", ::std::stringify!($name));
                    ::fontheight_core::::WordList {
                        name: ::std::string::String::from(
                            ::std::stringify!($name),
                        ),
                        words: super::newline_delimited_words(
                            ::std::ops::Deref::deref(&RAW_STR),
                        ),
                    }
                });
        }
    };
}

// include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

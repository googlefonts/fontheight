#![allow(missing_docs)]

use static_lang_word_lists::LOOKUP_TABLE;

#[test]
fn metadata_loads() {
    LOOKUP_TABLE.values().for_each(|word_list| {
        // Do something that prompts the deserialisation
        let _ = word_list.name();
    });
}

#[test]
fn word_lists_decompress() {
    LOOKUP_TABLE.values().for_each(|word_list| {
        let _ = word_list.iter().next();
    });
}

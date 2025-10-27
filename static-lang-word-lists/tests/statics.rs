#![allow(missing_docs)]

use static_lang_word_lists::ALL_WORD_LISTS;

#[test]
fn word_lists_decompress() {
    ALL_WORD_LISTS.iter().for_each(|word_list| {
        let _ = word_list.iter().next();
    });
}

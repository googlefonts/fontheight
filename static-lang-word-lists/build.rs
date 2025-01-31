use std::{
    env, fs,
    fs::{File, OpenOptions},
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use brotli::enc::{
    backward_references::BrotliEncoderMode, BrotliEncoderParams,
};

// FIXME: change branch to `main` before merging
const BASE_URL: &str = "https://raw.githubusercontent.com/googlefonts/fontheight/refs/heads/word-list-crate/static-lang-word-lists/data";

fn main() {
    let codegen_path = out_dir_path("codegen.rs");
    let mut codegen_file = open_path(&codegen_path);

    let latin_path = download_validate_compress("diffenator/Latin.txt");

    writeln!(
        &mut codegen_file,
        r#"
        wordlist! {{
            name: DiffenatorLatin,
            bytes: include_bytes!(r"{}"),
        }}"#,
        latin_path.display(),
    )
    .unwrap();
}

fn download_validate_compress(relative_path: &str) -> PathBuf {
    // Download & validate
    let response = minreq::get(format!("{BASE_URL}/{relative_path}"))
        .send()
        .unwrap_or_else(|err| panic!("failed to get {relative_path}: {err}"));
    assert_eq!(
        response.status_code, 200,
        "failed to get {relative_path}: {}",
        response.status_code
    );
    let bytes = response.into_bytes();
    assert!(
        std::str::from_utf8(&bytes).is_ok(),
        "{relative_path} was not UTF-8"
    );

    // Compress & write to disk
    let br_path = out_dir_path(relative_path).with_extension("txt.br");
    let mut br_file = open_path(&br_path);

    let mut cursor = Cursor::new(bytes.as_slice());
    brotli::BrotliCompress(&mut cursor, &mut br_file, &BrotliEncoderParams {
        mode: BrotliEncoderMode::BROTLI_MODE_TEXT,
        ..Default::default()
    })
    .expect("failed to read remote & compress");

    br_path
}

fn out_dir_path(name: &str) -> PathBuf {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    PathBuf::from(out_dir).join(name)
}

fn open_path(path: &Path) -> File {
    let Some(parent) = path.parent() else {
        unreachable!(
            "open_path will always be called on a file with a parent directory"
        );
    };
    fs::create_dir_all(parent).expect("failed to create parent directories");
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .unwrap_or_else(|err| {
            panic!("unable to open output file {}: {err}", path.display())
        })
}

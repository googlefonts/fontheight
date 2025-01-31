use std::{
    env, fs,
    fs::{File, OpenOptions},
    io::{Cursor, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use brotli::enc::{
    backward_references::BrotliEncoderMode, BrotliEncoderParams,
};

// FIXME: change branch to `main` before merging
const BASE_URL: &str = "https://raw.githubusercontent.com/googlefonts/fontheight/refs/heads/word-list-crate/static-lang-word-lists/data";

// Provides WORD_LISTS: &[(&str, &str)] for word list name and relative paths
// See egg.py for how this code is generated
include!("chicken.rs");

fn main() {
    println!("cargo::rerun-if-changed=chicken.rs");
    println!("cargo::rerun-if-changed=build.rs");

    let codegen_path = out_dir_path("codegen.rs");
    let codegen_file = Arc::new(Mutex::new(open_path(&codegen_path)));

    thread::scope(|s| {
        WORD_LISTS.iter().copied().for_each(|(name, path)| {
            let codegen_file = codegen_file.clone();
            s.spawn(move || {
                let br_path = download_validate_compress(path);
                let mut codegen_file = codegen_file.lock().unwrap();
                writeln!(
                    &mut codegen_file,
                    r#"
                    wordlist! {{
                        name: {name},
                        bytes: include_bytes!(r"{}"),
                    }}"#,
                    br_path.display(),
                )
                .unwrap();
            });
        });
    });
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

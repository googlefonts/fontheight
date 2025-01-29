use std::{
    env,
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use brotli::enc::{
    backward_references::BrotliEncoderMode, BrotliEncoderParams,
};

// FIXME: change branch to `main` before merging
const BASE_URL: &str = "https://raw.githubusercontent.com/googlefonts/fontheight/refs/heads/word-list-crate/static-lang-word-lists/data";

fn main() {
    let brotli_params = BrotliEncoderParams {
        mode: BrotliEncoderMode::BROTLI_MODE_TEXT,
        ..Default::default()
    };

    let codegen_path = out_dir_path("codegen.rs");
    let mut codegen_file = open_path(&codegen_path);

    let latin_path = out_dir_path("latin.txt.br");
    let mut latin_file = open_path(&latin_path);

    // TODO: check data as it comes in? Could save UTF-8 validate at runtime
    let mut latin = minreq::get(format!("{BASE_URL}/diffenator/Latin.txt"))
        .send_lazy()
        .expect("failed to get remote word list");
    assert_eq!(latin.status_code, 200);
    brotli::BrotliCompress(&mut latin, &mut latin_file, &brotli_params)
        .expect("failed to read remote & compress");

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

fn out_dir_path(name: &str) -> PathBuf {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    PathBuf::from(out_dir).join(name)
}

fn open_path(path: &Path) -> File {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap_or_else(|err| {
            panic!("unable to open output file {}: {err}", path.display())
        })
}

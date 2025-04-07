use std::{
    env, fs,
    fs::{File, OpenOptions},
    io::{Cursor, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
};

use brotli::enc::{
    backward_references::BrotliEncoderMode, BrotliEncoderParams,
};
use heck::ToShoutySnakeCase;

const BASE_URL: &str = "https://raw.githubusercontent.com/googlefonts/fontheight/refs/heads/main/static-lang-word-lists/data";

// Provides WORD_LISTS: &[(&str, &str)] for word list name and relative paths
// See egg.py for how this code is generated
include!("chicken.rs");

fn main() {
    println!("cargo::rerun-if-changed=chicken.rs");
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=STATIC_LANG_WORD_LISTS_LOCAL");

    if option_env!("STATIC_LANG_WORD_LISTS_LOCAL").is_some() {
        println!("cargo::warning=building from local files");
    }

    let word_list_path = out_dir_path("word_list_codegen.rs");
    let word_list_file = Mutex::new(open_path(&word_list_path));
    let map_path = out_dir_path("map_codegen.rs");
    let mut map_file = open_path(&map_path);

    writeln!(
        &mut map_file,
        "pub static LOOKUP_TABLE: ::phf::Map<&'static str, &'static \
         ::std::sync::LazyLock<WordList>> = ::phf::phf_map! {{"
    )
    .unwrap_or_else(|err| panic!("failed to write to map_codeden.rs: {err}"));
    let map_file = Mutex::new(map_file);

    thread::scope(|s| {
        // Bind references to names so they can be copied to each spawned thread
        let codegen_file = &word_list_file;
        let map_file = &map_file;
        WORD_LISTS.iter().copied().for_each(|(name, path)| {
            s.spawn(move || {
                let ident = name.to_shouty_snake_case();
                let bytes =
                    if option_env!("STATIC_LANG_WORD_LISTS_LOCAL").is_none() {
                        download_validate(path)
                    } else {
                        let repo_path = Path::new("data").join(path);
                        fs::read(&repo_path).unwrap_or_else(|err| {
                            panic!(
                                "failed to read local word list file {}: {err}",
                                repo_path.display()
                            );
                        })
                    };
                let br_path = compress(&bytes, path);

                let mut codegen_file = codegen_file.lock().unwrap();
                writeln!(
                    &mut codegen_file,
                    r#"
                    wordlist! {{
                        ident: {ident},
                        name: {name},
                        bytes: include_bytes!(r"{}"),
                    }}"#,
                    br_path.display(),
                )
                .unwrap_or_else(|err| {
                    panic!("failed to write to word_list_codegen.rs: {err}");
                });

                let mut map_file = map_file.lock().unwrap();
                writeln!(&mut map_file, r#"    "{name}" => &{ident},"#)
                    .unwrap_or_else(|err| {
                        panic!("failed to write to map_codeden.rs: {err}")
                    });
            });
        });
    });

    let mut map_file = map_file.into_inner().unwrap();
    writeln!(&mut map_file, "}};").unwrap_or_else(|err| {
        panic!("failed to write to map_codeden.rs: {err}")
    });
}

fn download_validate(relative_path: &str) -> Vec<u8> {
    let response = minreq::get(format!("{BASE_URL}/{relative_path}"))
        .send()
        .unwrap_or_else(|err| {
            panic!("failed to fetch {relative_path} from GitHub: {err}");
        });
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
    bytes
}

fn compress(bytes: &[u8], relative_path: &str) -> PathBuf {
    let br_path = out_dir_path(relative_path).with_extension("txt.br");
    let mut br_file = open_path(&br_path);

    let mut cursor = Cursor::new(bytes);
    brotli::BrotliCompress(&mut cursor, &mut br_file, &BrotliEncoderParams {
        mode: BrotliEncoderMode::BROTLI_MODE_TEXT,
        ..Default::default()
    })
    .unwrap_or_else(|err| panic!("failed to compress {relative_path}: {err}"));

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

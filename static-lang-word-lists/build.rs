#![allow(missing_docs)]

use std::{
    env,
    ffi::OsStr,
    fs,
    fs::{File, OpenOptions},
    io,
    io::{Cursor, Write},
    path::{Component, Path, PathBuf},
    sync::Mutex,
    thread,
};

use brotli::enc::{
    BrotliEncoderParams, backward_references::BrotliEncoderMode,
};
use heck::ToShoutySnakeCase;
use zip::ZipArchive;

// Provides WORD_LISTS: &[(&str, &str, &str)] for word list name, metadata name,
// and relative paths. See egg.py for how this code is generated
include!("chicken.rs");

static IS_DOCS_RS: bool = option_env!("DOCS_RS").is_some();
static LOCAL_BUILD: bool =
    option_env!("STATIC_LANG_WORD_LISTS_LOCAL").is_some();

fn main() {
    println!("cargo::rerun-if-changed=chicken.rs");
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=STATIC_LANG_WORD_LISTS_LOCAL");

    let word_list_path = out_dir_path("word_list_codegen.rs");
    let word_list_file = Mutex::new(open_path(&word_list_path));
    let map_path = out_dir_path("map_codegen.rs");
    let mut map_file = open_path(&map_path);

    let wordlist_source_dir = match (LOCAL_BUILD, IS_DOCS_RS) {
        // By default, download the word lists
        (false, false) => download_repo_word_lists(),
        // On docs.rs we have no network access, so we stub everything.
        // wordlist_source_dir isn't accessed
        (_, true) => Default::default(),
        // For local development, you can opt to build from local files
        (true, _) => {
            println!("cargo::warning=building from local files");
            PathBuf::from("data")
        },
    };

    writeln!(
        &mut map_file,
        r#"#[doc = "A lookup map for the crate-provided [`WordList`]s. Maps their names to the corresponding static [`WordList`]."]
        pub static LOOKUP_TABLE: ::phf::Map<&'static str, &'static crate::WordList> =
            ::phf::phf_map! {{"#
    )
    .unwrap_or_else(|err| panic!("failed to write to map_codeden.rs: {err}"));
    let map_file = Mutex::new(map_file);

    thread::scope(|s| {
        // Bind references to names so they can be copied to each spawned thread
        let wordlist_source_dir = wordlist_source_dir.as_path();
        let codegen_file = &word_list_file;
        let map_file = &map_file;
        WORD_LISTS
            .iter()
            .copied()
            .for_each(|(name, metadata_file, path)| {
                if IS_DOCS_RS {
                    let ident = name.to_shouty_snake_case();
                    let mut codegen_file = codegen_file.lock().unwrap();
                    writeln!(
                        &mut codegen_file,
                        "/// The {ident} word list.
                        ///
                        /// Compiled into the binary compressed with Brotli, \
                         decompressed at
                        /// runtime.
                        pub static {ident}: crate::WordList = \
                         crate::WordList::stub();",
                    )
                    .unwrap_or_else(|err| {
                        panic!(
                            "failed to write to word_list_codegen.rs: {err}"
                        );
                    });
                    let mut map_file = map_file.lock().unwrap();
                    writeln!(&mut map_file, r#"    "{name}" => &{ident},"#)
                        .unwrap_or_else(|err| {
                            panic!("failed to write to map_codeden.rs: {err}")
                        });
                    return;
                }
                s.spawn(move || {
                    let metadata_content = String::from_utf8(get_a_file(
                        metadata_file,
                        wordlist_source_dir,
                    ))
                    .expect("metadata file was not UTF-8");
                    let ident = name.to_shouty_snake_case();
                    let bytes = get_a_file(path, wordlist_source_dir);
                    // Validate the bytes are UTF-8 now so we don't need to at
                    // runtime
                    str::from_utf8(&bytes)
                        .expect("word list should be valid UTF-8");
                    let br_path = compress(&bytes, path);

                    let mut codegen_file = codegen_file.lock().unwrap();
                    writeln!(
                        &mut codegen_file,
                        r##"wordlist! {{
                            ident: {ident},
                            metadata: r#"{metadata_content}"#,
                            bytes: include_bytes!(r"{}"),
                        }}"##,
                        br_path.display(),
                    )
                    .unwrap_or_else(|err| {
                        panic!(
                            "failed to write to word_list_codegen.rs: {err}"
                        );
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

fn get_a_file(path: &str, data_dir: &Path) -> Vec<u8> {
    let repo_path = data_dir.join(path);
    fs::read(&repo_path).unwrap_or_else(|err| {
        panic!(
            "failed to read local word list file {}: {err}",
            repo_path.display()
        );
    })
}

fn download_repo_word_lists() -> PathBuf {
    let is_fontheight =
        option_env!("GITHUB_REPOSITORY") == Some("googlefonts/fontheight");
    let git_ref = is_fontheight
        .then_some(option_env!("GITHUB_HEAD_REF"))
        .flatten()
        .filter(|val| !val.is_empty())
        .unwrap_or("main");
    let url =
        format!("https://github.com/googlefonts/fontheight/zipball/{git_ref}");

    let response = minreq::get(&url).send().unwrap_or_else(|err| {
        panic!("failed to download fontheight repo from GitHub: {err}");
    });
    assert_eq!(
        response.status_code, 200,
        "failed to download repo: {} from {url}",
        response.status_code
    );
    let bytes = response.into_bytes();

    let out_dir = out_dir_path("static-lang-word-lists");
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).unwrap_or_else(|err| {
            panic!("failed to read repo zip archive: {err}");
        });
    for index in 0..archive.len() {
        let compressed_entry = archive.by_index_raw(index).unwrap();
        if !compressed_entry.is_file() {
            continue;
        }
        let Some(file_name) = compressed_entry.enclosed_name() else {
            continue;
        };

        // Check path is relevant
        let mut components = file_name.components();
        components.next(); // Drop root `googlefonts-fontheight-<hash>`
        if components.next()
            != Some(Component::Normal(OsStr::new("static-lang-word-lists")))
        {
            continue;
        }
        if components.next() != Some(Component::Normal(OsStr::new("data"))) {
            continue;
        }

        // Now we know we want it, decompress it
        drop(compressed_entry);
        let mut decompressed_entry = archive.by_index(index).unwrap();

        let final_path = out_dir
            .components()
            .chain(file_name.components().skip(2))
            .collect::<PathBuf>();
        fs::create_dir_all(final_path.parent().unwrap()).unwrap_or_else(
            |err| {
                panic!(
                    "failed to create parent directory for {}: {err}",
                    final_path.display(),
                );
            },
        );
        let mut out_file = File::create(&final_path).unwrap_or_else(|err| {
            panic!("failed to create file {}: {err}", final_path.display());
        });
        io::copy(&mut decompressed_entry, &mut out_file).unwrap_or_else(
            |err| {
                panic!(
                    "failed to copy file {} in {}: {err}",
                    file_name.display(),
                    out_dir.display(),
                );
            },
        );
    }
    out_dir.join("data")
}

fn compress(bytes: &[u8], relative_path: &str) -> PathBuf {
    let br_path = out_dir_path(relative_path).with_extension("txt.br");
    let mut br_file = open_path(&br_path);

    let mut cursor = Cursor::new(bytes);
    brotli::BrotliCompress(&mut cursor, &mut br_file, &BrotliEncoderParams {
        mode: BrotliEncoderMode::BROTLI_MODE_TEXT,
        quality: if env::var("PROFILE").as_deref() == Ok("debug") {
            8
        } else {
            11
        },
        size_hint: bytes.len(),
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

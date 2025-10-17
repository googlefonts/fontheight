#![allow(missing_docs)]

use std::{
    env,
    ffi::OsStr,
    fs,
    fs::{File, OpenOptions},
    io,
    io::Cursor,
    path::{Component, Path, PathBuf},
    thread,
};

use brotli::enc::{
    BrotliEncoderParams, backward_references::BrotliEncoderMode,
};
use zip::ZipArchive;

// Provides WORD_LISTS: &[&str] for word list relative path
include!("chicken.rs");

static IS_DOCS_RS: bool = option_env!("DOCS_RS").is_some();
static LOCAL_BUILD: bool =
    option_env!("STATIC_LANG_WORD_LISTS_LOCAL").is_some();

fn main() {
    println!("cargo::rerun-if-changed=chicken.rs");
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=STATIC_LANG_WORD_LISTS_LOCAL");

    if IS_DOCS_RS {
        // docs.rs doesn't allow network connection when building, so we don't
        // have access to the word list files, ergo there's nothing to compress.
        // src/declarations.rs has conditional compilation that stubs word lists
        // for docs.rs specifically, so everything will still build
        return;
    }

    let word_list_source_dir = match LOCAL_BUILD {
        // By default, download the word lists
        false => download_repo_word_lists(),
        // For local development, you can opt to build from local files
        true => {
            println!("cargo::warning=building from local files");
            PathBuf::from("data")
        },
    };

    // This speeds up debug builds significantly but still does a good job of
    // reducing size
    let compression_level = if env::var("PROFILE").as_deref() == Ok("debug") {
        8
    } else {
        11
    };

    thread::scope(|s| {
        // Bind references to names so they can be copied to each spawned thread
        let wordlist_source_dir = word_list_source_dir.as_path();
        WORD_LISTS.iter().copied().for_each(|rel_path| {
            s.spawn(move || {
                let bytes = get_a_file(rel_path, wordlist_source_dir);
                // Validate the bytes are UTF-8 now so we don't need to at
                // runtime
                str::from_utf8(&bytes)
                    .expect("word list should be valid UTF-8");
                compress(&bytes, rel_path, compression_level);
            });
        });
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

fn compress(
    bytes: &[u8],
    relative_path: &str,
    compression_level: u8,
) -> PathBuf {
    let br_path = out_dir_path(relative_path).with_extension("txt.br");
    let mut br_file = open_path(&br_path);

    let mut cursor = Cursor::new(bytes);
    brotli::BrotliCompress(&mut cursor, &mut br_file, &BrotliEncoderParams {
        mode: BrotliEncoderMode::BROTLI_MODE_TEXT,
        quality: compression_level as i32,
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

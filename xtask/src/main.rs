#![allow(missing_docs)]

use std::{env, ffi::OsStr, fs, path::PathBuf};

use anyhow::{Context, anyhow};
use pico_args::Arguments;
use proc_macro2::TokenStream;
use serde::Deserialize;
use walkdir::{DirEntry, WalkDir};

mod slwl;

fn main() -> anyhow::Result<()> {
    let mut args = Arguments::from_env();

    match args.subcommand()?.ok_or(anyhow!("missing task"))?.as_str() {
        "slwl" => slwl::main(args),
        unknown => Err(anyhow!("unknown task: {unknown}")),
    }
}

// Simplified version of `static_lang_word_lists::word_lists::WordListMetadata`.
// Keep in sync
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WordListMetadata {
    name: String,
    script: Option<String>,
    language: Option<String>,
}

fn workspace_root() -> PathBuf {
    env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .expect("xtask run without Cargo")
        .parent()
        .unwrap()
        .to_owned()
}

fn is_metadata_toml(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry.path().extension() == Some(OsStr::new("toml"))
}

fn load_all_metadata() -> anyhow::Result<Vec<(PathBuf, WordListMetadata)>> {
    let data_dir = workspace_root().join("static-lang-word-lists/data");
    let mut metadatas = WalkDir::new(&data_dir)
        .into_iter()
        .filter_map(|de_res| {
            match de_res {
                Ok(entry) if is_metadata_toml(&entry) => {
                    Some(entry.into_path())
                },
                // Irrelevant stuff
                Ok(_) => None,
                Err(err) => {
                    if let Some(path) = err.path()
                        && (path.is_dir()
                            || path.extension() == Some(OsStr::new("toml")))
                    {
                        // TOML file or directory; i.e. important, so we panic
                        // (because emitting a Result::Err from here would be a
                        // pain, and this is unlikely to happen)
                        panic!("{err}");
                    }
                    // If we didn't hit the above if statement, the error
                    // probably isn't important and we can carry on, but we'll
                    // let the user know because we're considerate
                    eprintln!("warning: {err}");
                    None
                },
            }
        })
        .map(
            |metadata_toml| -> anyhow::Result<(PathBuf, WordListMetadata)> {
                let bytes = fs::read(&metadata_toml)
                    .context("failed to read metadata")?;
                let metadata = toml::from_slice(&bytes)
                    .context("failed to parse metadata")?;
                Ok((metadata_toml, metadata))
            },
        )
        .collect::<anyhow::Result<Vec<_>>>()?;
    metadatas.sort_by(|(path_a, _), (path_b, _)| path_a.cmp(path_b));
    Ok(metadatas)
}

fn format_tokens(stream: TokenStream) -> anyhow::Result<String> {
    syn::parse2(stream)
        .map(|file| prettyplease::unparse(&file))
        .context("failed to parse TokenStream with syn")
}

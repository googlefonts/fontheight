#![allow(missing_docs)]

use std::{env, ffi::OsStr, fs, path::PathBuf};

use anyhow::{Context, anyhow};
use pico_args::Arguments;
use proc_macro2::TokenStream;
use static_lang_word_lists::WordListMetadata;
use walkdir::{DirEntry, WalkDir};

mod slwl;

fn main() -> anyhow::Result<()> {
    let mut args = Arguments::from_env();

    match args.subcommand()?.ok_or(anyhow!("missing task"))?.as_str() {
        "slwl" => slwl::main(args),
        unknown => Err(anyhow!("unknown task: {unknown}")),
    }
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
                Err(why) => {
                    eprintln!(
                        "failed to explore part of \
                         static-lang-word-lists/data: {why}"
                    );
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

use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::Context;
use clap::Parser;
use env_logger::Env;
use fontheight_core::Reporter;
use log::{error, info, LevelFilter};
use static_lang_word_lists::DIFFENATOR_LATIN;

fn main() -> ExitCode {
    env_logger::builder()
        .filter_level(if cfg!(debug_assertions) {
            LevelFilter::Debug
        } else {
            LevelFilter::Warn
        })
        .parse_env(Env::new().filter("FONTHEIGHT_LOG"))
        .init();
    match _main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            error!("{why}");
            ExitCode::FAILURE
        },
    }
}

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// The TTF to analyze
    font_path: PathBuf,
}

fn _main() -> anyhow::Result<()> {
    let args = Args::parse();

    let font_bytes =
        fs::read(&args.font_path).context("failed to read font file")?;

    let mut reporter = Reporter::new(&font_bytes)?;
    let locations = reporter.interesting_locations();
    let reports = reporter
        .check_location(&locations[0], &DIFFENATOR_LATIN)?
        .collect::<Vec<_>>();

    info!("{reports:#?}");
    Ok(())
}

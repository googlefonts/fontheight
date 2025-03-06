use std::{fs, iter, path::PathBuf, process::ExitCode};

use anyhow::Context;
use clap::Parser;
use env_logger::Env;
use fontheight_core::{Exemplars, Reporter};
use log::{debug, error, info, LevelFilter};

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

    /// The number of words to log
    #[arg(short, long)]
    results: usize,
}

fn _main() -> anyhow::Result<()> {
    let args = Args::parse();

    let font_bytes =
        fs::read(&args.font_path).context("failed to read font file")?;

    let reporter = Reporter::new(&font_bytes)?;
    let locations = reporter.interesting_locations();
    let reports = locations
        .iter()
        .flat_map(|location| {
            static_lang_word_lists::LOOKUP_TABLE
                .values()
                .zip(iter::repeat(location))
        })
        .map(|(word_list, location)| -> anyhow::Result<Exemplars> {
            debug!("checking {} at {location:?}", word_list.name());
            let exemplars = reporter
                .check_location(location, word_list)?
                .collect_min_max_extremes(args.results);
            Ok(exemplars)
        })
        .collect::<Vec<_>>();

    info!("{reports:#?}");
    Ok(())
}

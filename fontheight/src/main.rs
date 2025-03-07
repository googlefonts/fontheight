use std::{fs, iter, path::PathBuf, process::ExitCode, time::Instant};

use anyhow::Context;
use clap::Parser;
use env_logger::Env;
use fontheight_core::{Exemplars, Reporter};
use log::{debug, error, info, LevelFilter};
use rayon::prelude::*;

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
    #[arg(short = 'n', long, default_value_t = 5)]
    results: usize,
}

fn _main() -> anyhow::Result<()> {
    let args = Args::parse();

    let font_bytes =
        fs::read(&args.font_path).context("failed to read font file")?;

    let start = Instant::now();
    let reporter = Reporter::new(&font_bytes)?;
    let locations = reporter.interesting_locations();
    // TODO: an equivalent of Report from fontheight-wheel
    // TODO: prune empty exemplars (unsupported scripts)
    let reports = locations
        .iter()
        .flat_map(|location| {
            static_lang_word_lists::LOOKUP_TABLE
                .values()
                .zip(iter::repeat(location))
        })
        .par_bridge()
        .map(|(word_list, location)| -> anyhow::Result<Exemplars> {
            let exemplars = reporter
                .check_location(location, word_list)?
                .par_collect_min_max_extremes(args.results);
            debug!("finished checking {} at {location:?}", word_list.name());
            Ok(exemplars)
        })
        .collect::<Result<Vec<_>, _>>()?;

    info!("{reports:#?}");
    info!("Took {:?}", start.elapsed());
    Ok(())
}

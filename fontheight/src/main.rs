mod fmt;

use std::{fs, iter, path::PathBuf, process::ExitCode, time::Instant};

use anyhow::{bail, Context};
use clap::Parser;
use env_logger::Env;
use fontheight_core::Reporter;
use log::{debug, error, info, LevelFilter};
use rayon::prelude::*;

use crate::fmt::{FormatReport, OutputFormat};

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
    /// The TTF(s) to analyze
    font_path: Vec<PathBuf>,

    /// The number of words to log
    #[arg(short = 'n', long, default_value_t = 5)]
    results: usize,

    /// The number of words from each list to test
    // TODO: an --all flag
    #[arg(long = "words", default_value_t = 25)]
    words_per_list: usize,
}

fn _main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.font_path.is_empty() {
        bail!("must provide one or more FONT_PATHs");
    }

    args.font_path
        .iter()
        .try_for_each(|font_path| -> anyhow::Result<()> {
            let font_bytes =
                fs::read(font_path).context("failed to read font file")?;

            let start = Instant::now();
            let reporter = Reporter::new(&font_bytes)?;
            let locations = reporter.interesting_locations();

            let reports = locations
                .iter()
                .flat_map(|location| {
                    static_lang_word_lists::LOOKUP_TABLE
                        .values()
                        .zip(iter::repeat(location))
                })
                .par_bridge()
                .map(|(word_list, location)| -> anyhow::Result<_> {
                    let report = reporter
                        .par_check_location(
                            location,
                            word_list,
                            args.words_per_list,
                            args.results,
                        )?
                        .to_report(location, word_list);
                    debug!(
                        "finished checking {} at {location:?}",
                        word_list.name()
                    );
                    Ok(report)
                })
                .filter(|report_res| {
                    report_res
                        .as_ref()
                        .map_or(true, |report| !report.exemplars.is_empty())
                })
                .collect::<Result<Vec<_>, _>>()?;

            let took = start.elapsed();
            println!("{}:", font_path.display());
            reports.iter().for_each(|report| {
                println!("{}", report.format(OutputFormat::Human));
            });
            info!("Took {took:?}");
            Ok(())
        })
}

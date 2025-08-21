#![allow(missing_docs)]

mod fmt;

use std::{fs, iter, path::PathBuf, process::ExitCode, time::Instant};

use anyhow::{Context, bail};
use clap::Parser;
use env_logger::Env;
use fontheight::Reporter;
use log::{LevelFilter, debug, error, info, warn};
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

    /// The number of words from each list to test [default: all words]
    // TODO: an --all flag
    #[arg(short = 'k', long = "words")]
    words_per_list: Option<usize>,
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
            info!(
                "Found {} interesting locations in {}",
                locations.len(),
                font_path.display(),
            );

            let instances = locations
                .par_iter()
                .map(|location| reporter.instance(location))
                .collect::<Result<Vec<_>, _>>()?;

            if instances.len() >= 100 && args.words_per_list.is_none() {
                warn!(
                    "Testing {} instances with all words is probably going to \
                     take a while. Consider passing -k/--words to limit the \
                     number of words being checked",
                    instances.len()
                );
            }

            let reports = instances
                .iter()
                .flat_map(|instance| {
                    static_lang_word_lists::LOOKUP_TABLE
                        .values()
                        .zip(iter::repeat(instance))
                })
                .par_bridge()
                .map(|(word_list, instance)| -> anyhow::Result<_> {
                    let report = instance.par_check(
                        word_list,
                        args.words_per_list,
                        args.results,
                    )?;
                    debug!(
                        "finished checking {} at {:?}",
                        word_list.name(),
                        report.location
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

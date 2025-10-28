#![allow(missing_docs)]

use std::{
    fs,
    fs::OpenOptions,
    io::{Write, stdout},
    iter,
    path::PathBuf,
    process::ExitCode,
    time::Instant,
};

use anyhow::{Context, bail};
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use env_logger::Env;
use fmt::{FormatReport, OutputFormat};
use fontheight::Reporter;
use log::{error, info, warn};
use rayon::prelude::*;

mod fmt;

fn main() -> ExitCode {
    match _main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            error!("{why}");
            ExitCode::FAILURE
        },
    }
}

// Default to debug logs on debug builds, info otherwise
#[cfg(debug_assertions)]
type FontheightVerbosity = Verbosity<clap_verbosity_flag::DebugLevel>;
#[cfg(not(debug_assertions))]
type FontheightVerbosity = Verbosity<clap_verbosity_flag::InfoLevel>;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// The TTF(s) to analyze
    #[arg(required = true)]
    font_path: Vec<PathBuf>,

    /// The number of words to log
    #[arg(short = 'n', long, default_value_t = 5)]
    results: usize,

    /// The number of words from each list to test [default: all words]
    #[arg(short = 'k', long = "words")]
    words_per_list: Option<usize>,

    #[command(flatten)]
    verbosity: FontheightVerbosity,

    /// Write the reports into the given path.
    /// Will print to stdout if not specified
    #[arg(short, long = "output")]
    output_path: Option<PathBuf>,

    /// Output all the reports into a single HTML file
    #[arg(long)]
    html: bool,
}

fn _main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.font_path.len() > 1 && args.html {
        bail!("you can't pass multiple fonts if using --html");
    }

    env_logger::builder()
        .filter_level(args.verbosity.into())
        .parse_env(Env::new().filter("FONTHEIGHT_LOG"))
        .init();

    let mut output: Box<dyn Write> = match &args.output_path {
        None => Box::new(stdout().lock()),
        Some(path) => {
            let handle = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(path)
                .context("failed to open output file")?;
            Box::new(handle)
        },
    };

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
                    static_lang_word_lists::ALL_WORD_LISTS
                        .iter()
                        .zip(iter::repeat(instance))
                })
                .par_bridge()
                .map(|(word_list, instance)| -> anyhow::Result<_> {
                    let report = instance.par_check(
                        word_list,
                        args.words_per_list,
                        args.results,
                    )?;
                    info!(
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
            info!("{} took {took:?}", font_path.display());

            if !args.html {
                writeln!(&mut output, "{}:", font_path.display())
                    .context("failed to write to output")?;
                reports
                    .iter()
                    .try_for_each(|report| {
                        writeln!(
                            &mut output,
                            "{}",
                            report.format(OutputFormat::Human)
                        )
                    })
                    .context("failed to write to output")?;
            } else {
                let html = fmt::html::format_all_reports(
                    &reports,
                    reporter.fontref(),
                )?;
                output
                    .write_all(html.as_bytes())
                    .context("failed to write to output")?;
            }
            Ok(())
        })
}

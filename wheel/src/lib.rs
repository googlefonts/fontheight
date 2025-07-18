use std::{fmt::Write, fs, iter, path::PathBuf};

use anyhow::{Context, anyhow};
use fontheight::{Exemplars, Report, Reporter, SimpleLocation, WordExtremes};
use pyo3::{Bound, PyResult, prelude::*, pymodule};

#[pyclass(name = "Report", frozen, get_all)]
#[derive(Debug, Clone)]
pub struct OwnedReport {
    location: SimpleLocation,
    word_list_name: String,
    exemplars: OwnedExemplars,
}

#[pymethods]
impl OwnedReport {
    fn __repr__(&self) -> String {
        let OwnedReport {
            location,
            word_list_name,
            exemplars,
        } = &self;
        format!(
            "Report(location={location:?}, \
             word_list_name=\"{word_list_name}\", exemplars={})",
            exemplars.__repr__()
        )
    }
}

impl From<Report<'_>> for OwnedReport {
    fn from(report: Report) -> Self {
        let Report {
            location,
            word_list,
            exemplars,
        } = report;
        OwnedReport {
            location: location.to_simple(),
            word_list_name: word_list.name().to_owned(),
            exemplars: exemplars.into(),
        }
    }
}

#[pyclass(name = "Exemplars", frozen, get_all)]
#[derive(Debug, Clone)]
pub struct OwnedExemplars {
    lowest: Vec<OwnedWordExtremes>,
    highest: Vec<OwnedWordExtremes>,
}

#[pymethods]
impl OwnedExemplars {
    fn __repr__(&self) -> String {
        let mut buf = String::from("Exemplars(lowest=[");

        self.lowest.iter().for_each(|low| {
            write!(&mut buf, "{}, ", low.__repr__()).unwrap();
        });
        buf.pop();
        buf.pop();

        buf.push_str("], highest=[");
        self.highest.iter().for_each(|high| {
            write!(&mut buf, "{}, ", high.__repr__()).unwrap();
        });
        buf.pop();
        buf.pop();
        buf.push_str("])");

        buf
    }
}

impl From<Exemplars<'_>> for OwnedExemplars {
    fn from(exemplars: Exemplars<'_>) -> Self {
        OwnedExemplars {
            lowest: exemplars
                .lowest()
                .iter()
                .map(OwnedWordExtremes::from)
                .collect(),
            highest: exemplars
                .highest()
                .iter()
                .map(OwnedWordExtremes::from)
                .collect(),
        }
    }
}

#[pyclass(name = "WordExtremes", frozen, get_all)]
#[derive(Debug, Clone)]
pub struct OwnedWordExtremes {
    word: String,
    lowest: f64,
    highest: f64,
}

#[pymethods]
impl OwnedWordExtremes {
    fn __repr__(&self) -> String {
        let OwnedWordExtremes {
            word,
            lowest,
            highest,
        } = &self;
        format!(
            "WordExtremes(word=\"{word}\", lowest={lowest}, highest={highest})"
        )
    }
}

impl From<&WordExtremes<'_>> for OwnedWordExtremes {
    fn from(report: &WordExtremes<'_>) -> Self {
        OwnedWordExtremes {
            word: report.word.to_owned(),
            lowest: report.extremes.lowest(),
            highest: report.extremes.highest(),
        }
    }
}

#[pyfunction]
pub fn get_min_max_extremes_from(
    path: PathBuf,
    k_words: Option<usize>,
    n_exemplars: usize,
) -> anyhow::Result<Vec<OwnedReport>> {
    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    get_min_max_extremes(&bytes, k_words, n_exemplars)
}

#[pyfunction]
pub fn get_min_max_extremes(
    font_bytes: &[u8],
    k_words: Option<usize>,
    n_exemplars: usize,
) -> anyhow::Result<Vec<OwnedReport>> {
    let reporter = Reporter::new(font_bytes)?;
    let locations = reporter.interesting_locations();
    locations
        .iter()
        .flat_map(|location| {
            static_lang_word_lists::LOOKUP_TABLE
                .values()
                .zip(iter::repeat(location))
        })
        .map(|(word_list, location)| -> anyhow::Result<OwnedReport> {
            let report = reporter
                .par_check_location(location, word_list, k_words, n_exemplars)?
                .to_report(location, word_list)
                .into();
            Ok(report)
        })
        .collect::<Result<Vec<_>, _>>()
}

// Internal API for sort_by_vertical_extremes.py
#[pyfunction(name = "_get_all_word_list_extremes")]
pub fn get_all_word_list_extremes(
    path: PathBuf,
    word_list: &str,
) -> anyhow::Result<Vec<OwnedWordExtremes>> {
    let font_bytes = fs::read(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let reporter = Reporter::new(&font_bytes)?;
    let locations = reporter.interesting_locations();
    let word_list = static_lang_word_lists::LOOKUP_TABLE
        .get(word_list)
        .ok_or(anyhow!("no word list named \"{word_list}\""))?;
    locations.iter().try_fold(
        Vec::new(),
        |mut acc, location| -> anyhow::Result<_> {
            let report_iter = reporter
                .check_location(location, word_list)?
                .map(|extremes| OwnedWordExtremes::from(&extremes));
            acc.extend(report_iter);
            Ok(acc)
        },
    )
}

#[pymodule]
fn libfontheight(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<OwnedReport>()?;
    module.add_class::<OwnedExemplars>()?;
    module.add_class::<OwnedWordExtremes>()?;
    module.add_function(wrap_pyfunction!(get_min_max_extremes, module)?)?;
    module
        .add_function(wrap_pyfunction!(get_min_max_extremes_from, module)?)?;
    module
        .add_function(wrap_pyfunction!(get_all_word_list_extremes, module)?)?;
    Ok(())
}

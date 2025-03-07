use std::{fmt::Write, fs, path::PathBuf};

use anyhow::Context;
use fontheight_core::{Exemplars, Reporter, SimpleLocation, WordExtremes};
use pyo3::{Bound, PyResult, prelude::*, pymodule};
use static_lang_word_lists::DIFFENATOR_LATIN;

#[pyclass(frozen, get_all)]
#[derive(Debug, Clone)]
struct Report {
    location: SimpleLocation,
    word_list_name: String,
    exemplars: OwnedExemplars,
}

#[pymethods]
impl Report {
    fn __repr__(&self) -> String {
        let Report {
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

#[pyclass(name = "Exemplars", frozen, get_all)]
#[derive(Debug, Clone)]
struct OwnedExemplars {
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
struct OwnedWordExtremes {
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
fn get_min_max_extremes_from(
    path: PathBuf,
    n: usize,
) -> anyhow::Result<Vec<Report>> {
    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    get_min_max_extremes(&bytes, n)
}

#[pyfunction]
fn get_min_max_extremes(
    font_bytes: &[u8],
    n: usize,
) -> anyhow::Result<Vec<Report>> {
    let reporter = Reporter::new(font_bytes)?;
    let locations = reporter.interesting_locations();
    locations
        .iter()
        .map(|location| -> anyhow::Result<Report> {
            let report_iter =
                reporter.check_location(location, &DIFFENATOR_LATIN)?;
            Ok(Report {
                location: location.to_simple(),
                word_list_name: DIFFENATOR_LATIN.name().to_owned(),
                exemplars: report_iter.collect_min_max_extremes(n).into(),
            })
        })
        .collect::<Result<Vec<_>, _>>()
}

#[pymodule]
fn fontheight(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Report>()?;
    module.add_class::<OwnedExemplars>()?;
    module.add_class::<OwnedWordExtremes>()?;
    module.add_function(wrap_pyfunction!(get_min_max_extremes, module)?)?;
    module
        .add_function(wrap_pyfunction!(get_min_max_extremes_from, module)?)?;
    Ok(())
}

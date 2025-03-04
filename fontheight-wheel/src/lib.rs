use fontheight_core::{Exemplars, Reporter, SimpleLocation, WordExtremes};
use pyo3::{Bound, PyResult, prelude::*, pymodule, types::PyBytes};
use static_lang_word_lists::DIFFENATOR_LATIN;

#[pyclass(frozen, get_all)]
#[derive(Debug, Clone)]
struct Report {
    location: SimpleLocation,
    word_list_name: String,
    exemplars: OwnedExemplars,
}

#[pyclass(name = "Exemplars", frozen, get_all)]
#[derive(Debug, Clone)]
struct OwnedExemplars {
    lowest: Vec<OwnedWordExtremes>,
    highest: Vec<OwnedWordExtremes>,
}

impl From<Exemplars<'_>> for OwnedExemplars {
    fn from(summary: Exemplars<'_>) -> Self {
        let Exemplars { lowest, highest } = summary;
        OwnedExemplars {
            lowest: lowest.into_iter().map(OwnedWordExtremes::from).collect(),
            highest: highest.into_iter().map(OwnedWordExtremes::from).collect(),
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

impl From<WordExtremes<'_>> for OwnedWordExtremes {
    fn from(report: WordExtremes<'_>) -> Self {
        OwnedWordExtremes {
            word: report.word.to_owned(),
            lowest: report.extremes.lowest(),
            highest: report.extremes.highest(),
        }
    }
}

#[pyfunction]
fn get_min_max_extremes(
    font_bytes: Py<PyBytes>,
    n: usize,
) -> anyhow::Result<Vec<Report>> {
    let bytes = Python::with_gil(|py| font_bytes.as_bytes(py));
    let reporter = Reporter::new(bytes)?;
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
    Ok(())
}

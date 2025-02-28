use std::collections::HashMap;

use fontheight_core::{Location, Report, Reporter, VerticalExtremes};
use pyo3::{Bound, PyResult, prelude::*, pymodule, types::PyBytes};
use static_lang_word_lists::DIFFENATOR_LATIN;

#[pyclass(frozen, get_all)]
struct FontheightReport {
    word: String,
    highest: f64,
    lowest: f64,
    location: HashMap<String, f32>,
}

impl FontheightReport {
    fn new(report: Report, location: &Location) -> Self {
        let Report { word, extremes } = report;
        let VerticalExtremes { highest, lowest } = extremes;
        FontheightReport {
            word: word.to_owned(),
            location: location
                .user_coords()
                .iter()
                .map(|(tag, &value)| (tag.to_string(), value))
                .collect(),
            highest,
            lowest,
        }
    }
}

#[pyfunction]
fn get_min_max_extremes(
    font_bytes: Py<PyBytes>,
    n: usize,
) -> anyhow::Result<Vec<FontheightReport>> {
    let bytes = Python::with_gil(|py| font_bytes.as_bytes(py));
    let mut reporter = Reporter::new(bytes)?;
    let locations = reporter.interesting_locations();
    let reports = reporter
        .check_location(&locations[0], &DIFFENATOR_LATIN)?
        .map(|report| FontheightReport::new(report, &locations[0]))
        .take(n)
        .collect::<Vec<_>>();

    Ok(reports)
}

#[pymodule]
fn fontheight(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<FontheightReport>()?;
    module.add_function(wrap_pyfunction!(get_min_max_extremes, module)?)?;
    Ok(())
}

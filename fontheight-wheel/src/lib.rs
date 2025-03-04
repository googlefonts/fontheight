use fontheight_core::{Report, ReportSummary, Reporter, SimpleLocation};
use pyo3::{Bound, PyResult, prelude::*, pymodule, types::PyBytes};
use static_lang_word_lists::DIFFENATOR_LATIN;

#[pyclass(frozen, get_all)]
#[derive(Debug, Clone)]
struct FontheightReport {
    location: SimpleLocation,
    word_list_name: String,
    exemplars: OwnedReportSummary,
}

#[pyclass(name = "ReportSummary", frozen, get_all)]
#[derive(Debug, Clone)]
struct OwnedReportSummary {
    lowest: Vec<OwnedReport>,
    highest: Vec<OwnedReport>,
}

impl From<ReportSummary<'_>> for OwnedReportSummary {
    fn from(summary: ReportSummary<'_>) -> Self {
        let ReportSummary { lowest, highest } = summary;
        OwnedReportSummary {
            lowest: lowest.into_iter().map(OwnedReport::from).collect(),
            highest: highest.into_iter().map(OwnedReport::from).collect(),
        }
    }
}

#[pyclass(name = "Report", frozen, get_all)]
#[derive(Debug, Clone)]
struct OwnedReport {
    word: String,
    lowest: f64,
    highest: f64,
}

impl From<Report<'_>> for OwnedReport {
    fn from(report: Report<'_>) -> Self {
        OwnedReport {
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
) -> anyhow::Result<Vec<FontheightReport>> {
    let bytes = Python::with_gil(|py| font_bytes.as_bytes(py));
    let reporter = Reporter::new(bytes)?;
    let locations = reporter.interesting_locations();

    locations
        .iter()
        .map(|location| -> anyhow::Result<FontheightReport> {
            let report_iter =
                reporter.check_location(location, &DIFFENATOR_LATIN)?;
            Ok(FontheightReport {
                location: location.to_simple(),
                word_list_name: DIFFENATOR_LATIN.name().to_owned(),
                exemplars: report_iter.collect_min_max_extremes(n).into(),
            })
        })
        .collect::<Result<Vec<_>, _>>()
}

#[pymodule]
fn fontheight(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<FontheightReport>()?;
    module.add_class::<OwnedReportSummary>()?;
    module.add_class::<OwnedReport>()?;
    module.add_function(wrap_pyfunction!(get_min_max_extremes, module)?)?;
    Ok(())
}

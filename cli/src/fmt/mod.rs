use std::fmt;

use fontheight::Report;

pub mod html;

#[derive(Debug, Copy, Clone)]
pub struct ReportFormatter<'a> {
    report: &'a Report<'a>,
    format: OutputFormat,
}

impl fmt::Display for ReportFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ReportFormatter { report, format } = *self;
        match format {
            OutputFormat::Human => {
                writeln!(
                    f,
                    "  {} @ {:?}:",
                    report.word_list.name(),
                    report.location,
                )?;
                writeln!(f, "    {} tallest words:", report.exemplars.len(),)?;
                report.exemplars.highest().iter().try_for_each(|exemplar| {
                    writeln!(
                        f,
                        "      \"{}\" => {}",
                        exemplar.word,
                        exemplar.extremes.highest(),
                    )
                })?;
                writeln!(f, "    {} lowest words:", report.exemplars.len(),)?;
                // Little bit of extra work as the formatter shouldn't leave a
                // trailing newline
                let last = report.exemplars.len() - 1;
                report.exemplars.lowest().iter().enumerate().try_for_each(
                    |(index, exemplar)| {
                        if index != last {
                            writeln!(
                                f,
                                "      \"{}\" => {}",
                                exemplar.word,
                                exemplar.extremes.lowest(),
                            )
                        } else {
                            write!(
                                f,
                                "      \"{}\" => {}",
                                exemplar.word,
                                exemplar.extremes.lowest(),
                            )
                        }
                    },
                )?;
            },
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum OutputFormat {
    Human,
}

pub trait FormatReport<'a> {
    fn format(&'a self, format: OutputFormat) -> ReportFormatter<'a>;
}

impl<'a> FormatReport<'a> for Report<'a> {
    fn format(&'a self, format: OutputFormat) -> ReportFormatter<'a> {
        ReportFormatter {
            report: self,
            format,
        }
    }
}

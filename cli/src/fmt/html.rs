use std::{fmt, fmt::Write};

use fontheight::{Location, Report, WordExtremes};
use log::debug;
use maud::{DOCTYPE, Escaper, Markup, Render, html};
use skrifa::{FontRef, Tag, raw::TableProvider};
use static_lang_word_lists::WordList;
use svg::node::element::{Line, SVG};

static CSS: &str = "\
body {
    margin: 1em;

    font-family: sans-serif;
}

h1 {
    text-align: center;
}

details {
    margin: 4rem 0;
}

summary h2 {
    display: inline;
}

ul.drawn {
    list-style: none;
    margin-left: 0;
    padding-left: 0;

    display: flex;
    flex-wrap: wrap;
    gap: 2rem;
}

.drawn figure {
    margin: 0;
}

.drawn figcaption {
    font-family: monospace;
    text-align: center;
}

.drawn svg {
    height: 256px;
    border: 1px grey dashed;
    padding: 1rem;
}";

struct Debug<T: fmt::Debug>(T);

impl<T: fmt::Debug> Render for Debug<T> {
    fn render_to(&self, output: &mut String) {
        let mut escaper = Escaper::new(output);
        write!(escaper, "{:?}", self.0).unwrap();
    }
}

fn get_relevant_base_record(
    font: &FontRef,
    word_list: &WordList,
) -> Option<(i16, i16)> {
    // TODO: should probably at least warn! for any errors that emerge here that
    //       we're not expecting
    let base = font.base().ok()?;
    let ot_script = word_list.script().map(iso15924_to_opentype)?;
    let relevant_script_record =
        base.horiz_axis()?.ok()?.base_script_list().ok().and_then(
            |base_script_list| {
                base_script_list
                    .base_script_records()
                    .iter()
                    .find(|record| record.base_script_tag == ot_script)
            },
        )?;

    let base_script = relevant_script_record
        .base_script(base.offset_data())
        .ok()?;
    let min_max = match word_list
        .language()
        .and_then(|lang| {
            base_script.base_lang_sys_records().iter().find(|record| {
                record.base_lang_sys_tag
                    == Tag::new_checked(lang.as_bytes()).unwrap()
            })
        })
        .map(|lang_record| {
            lang_record.min_max(base_script.offset_data()).unwrap()
        }) {
        None => {
            let min_max = base_script.default_min_max()?.ok()?;
            debug!(
                "found script BASE entry for {}",
                word_list.script().unwrap()
            );
            min_max
        },
        Some(min_max) => {
            debug!(
                "found language-specific BASE override for {}",
                word_list.language().unwrap()
            );
            min_max
        },
    };

    Some((
        min_max.max_coord()?.ok()?.coordinate(),
        min_max.min_coord()?.ok()?.coordinate(),
    ))
}

fn draw_svg(
    _word: &str,
    font: &FontRef,
    _location: &Location,
    word_list: &WordList,
) -> SVG {
    // TODO: proper bounds
    let x_min = 0;
    let x_max = 3000;
    let height = 3000f32;

    let os2 = font.os2().unwrap();
    let head = font.head().unwrap();
    let upm = head.units_per_em() as f32;

    let mut highs = vec![
        (os2.s_typo_ascender() as f32, "red"),
        (os2.us_win_ascent() as f32, "blue"),
        (head.y_max() as f32, "green"),
        (1900. / 2048. * upm, "pink"),
    ];
    let mut lows = vec![
        (os2.s_typo_descender() as f32, "red"),
        (os2.us_win_descent() as f32, "blue"),
        (head.y_min() as f32, "green"),
        (-500. / 2048. * upm, "pink"),
    ];

    if let Some((max, min)) = get_relevant_base_record(font, word_list) {
        highs.push((max as f32, "cyan"));
        lows.push((min as f32, "cyan"));
    }

    let (highest, _) = highs
        .iter()
        .max_by(|(value_a, _), (value_b, _)| f32::total_cmp(value_a, value_b))
        .copied()
        .unwrap();
    let (lowest, _) = lows
        .iter()
        .min_by(|(value_a, _), (value_b, _)| f32::total_cmp(value_a, value_b))
        .copied()
        .unwrap();

    let line = |y: f32, colour: &str| {
        Line::new()
            .set("x1", x_min)
            .set("y1", height - y)
            .set("x2", x_max)
            .set("y2", height - y)
            .set("stroke-width", 10)
            .set("stroke", colour)
    };

    highs
        .into_iter()
        .chain(lows)
        .fold(SVG::new(), |svg, (value, colour)| {
            svg.add(line(value, colour))
        })
        .set(
            "viewBox",
            format!("{x_min} 0 {} {}", x_max - x_min, highest - lowest),
        )
        .set("preserveAspectRatio", "meet")
    // TODO: add the word itself
}

fn draw_exemplar(
    exemplar: WordExtremes,
    source: &WordList,
    location: &Location,
    font: &FontRef,
) -> Markup {
    html! {
        li {
            figure {
                (draw_svg(exemplar.word, font, location, source))
                figcaption {
                    (exemplar.word) " (from " (source.name()) ")" br;
                    (Debug(location))
                }
            }
        }
    }
}

fn format_report<'a>(report: &'a Report<'a>, font: &FontRef) -> Markup {
    html! {
        details open {
            summary { h2 { (report.word_list.script().unwrap_or("Misc.")) } }
            ul.drawn {
                @for high_exemplar in report.exemplars.highest() {
                    (draw_exemplar(*high_exemplar, report.word_list, report.location, font))
                }
                @for low_exemplar in report.exemplars.lowest() {
                    (draw_exemplar(*low_exemplar, report.word_list, report.location, font))
                }
            }
        }
    }
}

pub fn format_all_reports<'a>(
    reports: &'a [Report<'a>],
    font: &FontRef,
) -> String {
    html! {
        (DOCTYPE)
        html {
            head {
                title { "Tall Glyphs" }
                meta charset="utf-8"
                style { (CSS) }
            }
            body {
                h1 { "Tall Glyphs" }
                p {
                    "Lines legend:" br;
                    span style="color: green" {
                        "green: [head.yMax, head.yMin]"
                    } br;
                    span style="color: red" {
                        "red: [os2.sTypoAscender, os2.sTypoDescender]" br;
                        "= clipping limit for Android"
                    } br;
                    span style="color: pink" {
                        "pink: [1900&frasl;2048&times;upem, &minus;500&frasl;2048&times;upem]"
                    } br;
                    span style="color: cyan" {
                        "cyan: BASE table entry for script (if present)"
                    } br;
                }
                @for report in reports { (format_report(report, font)) }
            }
        }
    }.into_string()
}

// https://github.com/simoncozens/autobase/blob/9887854fd7436d034c15bf5875686b7583536e76/autobase/src/utils.rs#L223-L248
fn iso15924_to_opentype(script: &str) -> Tag {
    match script {
        // Special cases: https://github.com/fonttools/fonttools/blob/3c1822544d608f87c41fc8fb9ba41ea129257aa8/Lib/fontTools/unicodedata/OTTags.py
        // Relevant specification: https://learn.microsoft.com/en-us/typography/opentype/spec/scripttags
        // SCRIPT_EXCEPTIONS
        "Hira" => Tag::new(b"kana"),
        "Hrkt" => Tag::new(b"kana"),
        "Laoo" => Tag::new(b"lao "),
        "Yiii" => Tag::new(b"yi  "),
        "Nkoo" => Tag::new(b"nko "),
        "Vaii" => Tag::new(b"vai "),
        // NEW_SCRIPT_TAGS
        "Beng" => Tag::new(b"bng2"),
        "Deva" => Tag::new(b"dev2"),
        "Gujr" => Tag::new(b"gjr2"),
        "Guru" => Tag::new(b"gur2"),
        "Knda" => Tag::new(b"knd2"),
        "Mlym" => Tag::new(b"mlm2"),
        "Orya" => Tag::new(b"ory2"),
        "Taml" => Tag::new(b"tml2"),
        "Telu" => Tag::new(b"tel2"),
        "Mymr" => Tag::new(b"mym2"),
        // General case
        _ => Tag::new_checked(script.to_lowercase().as_bytes()).unwrap(),
    }
}

use std::{cmp::Ordering, collections::BTreeMap, fmt, fmt::Write, ops::Neg};

use fontheight::{Location, Report, WordExtremes};
use harfrust::{ShaperData, ShaperInstance, UnicodeBuffer};
use harfshapedfa::{
    HarfRustShaperExt, ShapingMeta, utils::iso15924_to_opentype,
};
use log::debug;
use maud::{DOCTYPE, Escaper, Markup, PreEscaped, Render, html};
use skrifa::{
    FontRef, MetadataProvider, Tag,
    instance::Size,
    outline::{DrawSettings, OutlinePen, pen::SvgPen},
    raw::TableProvider,
};
use static_lang_word_lists::WordList;
use svg::node::element::{Line, Path, SVG};

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
    height: 100px;
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
    let ot_script = word_list.script().map(iso15924_to_opentype)?.ok()?;
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
    word: &str,
    font: &FontRef,
    location: &Location,
    word_list: &WordList,
) -> SVG {
    let shaper_data = ShaperData::new(font);
    let shaper_instance =
        ShaperInstance::from_variations(font, location.to_harfrust());
    let shaper = shaper_data
        .shaper(font)
        .instance(Some(&shaper_instance))
        .build();
    let shaping_meta = word_list
        .script()
        .map(|script| ShapingMeta::new(script, word_list.language(), &shaper))
        .transpose()
        .unwrap(); // TODO: error handling
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(word);
    // Default features are still included by default
    let glyph_buffer = match &shaping_meta {
        Some(meta) => shaper.shape_with_meta(meta, buffer, &[]),
        None => {
            buffer.guess_segment_properties();
            shaper.shape(buffer, &[])
        },
    };

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
        ((os2.us_win_descent() as f32).neg(), "blue"),
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

    let skrifa_location = location.to_skrifa(font);
    let outlines = font.outline_glyphs();
    let mut svg_pen = SvgPen::new();
    let (x_max, _) = glyph_buffer
        .glyph_infos()
        .iter()
        .zip(glyph_buffer.glyph_positions())
        .fold(
            (0f32, 0f32),
            |(advance_width, advance_height), (glyph_info, position)| {
                let glyph = outlines.get(glyph_info.glyph_id.into()).unwrap();
                let offset_x = advance_width + position.x_offset as f32;
                let offset_y = advance_height + position.y_offset as f32;
                let mut offset_svg_pen = OffsetPen {
                    offset_x,
                    offset_y,
                    inner: &mut svg_pen,
                };
                let mut flipped_offset_svg_pen = VerticalFlipPen {
                    height: highest,
                    inner: &mut offset_svg_pen,
                };
                glyph
                    .draw(
                        DrawSettings::unhinted(
                            Size::unscaled(),
                            &skrifa_location,
                        ),
                        &mut flipped_offset_svg_pen,
                    )
                    .unwrap();
                (
                    offset_x + position.x_advance as f32,
                    offset_y + position.y_advance as f32,
                )
            },
        );
    let word_svg = Path::new().set("d", svg_pen.to_string());

    let x_min = 0.;
    let height = highest;

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
        .add(word_svg)
}

fn draw_exemplar(
    exemplar: WordExtremes,
    source: &WordList,
    location: &Location,
    font: &FontRef,
) -> Markup {
    let svg = draw_svg(exemplar.word, font, location, source).to_string();
    html! {
        li {
            figure {
                (PreEscaped(svg))
                figcaption {
                    "\"" (exemplar.word) "\" (from " (source.name()) ")" br;
                    (Debug(location))
                }
            }
        }
    }
}

fn format_script_reports<'a>(
    script: &str,
    reports: &'a [&'a Report<'a>],
    font: &FontRef,
) -> Markup {
    html! {
        details open {
            summary { h2 { (script) } }
            @for report in reports {
                ul.drawn {
                    @for high_exemplar in report.exemplars.highest() {
                        (draw_exemplar(
                            *high_exemplar,
                            report.word_list,
                            report.location,
                            font,
                        ))
                    }
                    @for low_exemplar in report.exemplars.lowest() {
                        (draw_exemplar(
                            *low_exemplar,
                            report.word_list,
                            report.location,
                            font,
                        ))
                    }
                }
            }
        }
    }
}

pub fn format_all_reports<'a>(
    reports: &'a [Report<'a>],
    font: &FontRef,
) -> String {
    // Group on script and then present exemplars from word lists in order by
    // name
    let mut script_exemplars = BTreeMap::<&str, Vec<&Report>>::new();
    reports.iter().for_each(|report| {
        // ZWSP at the start of Unknown so it gets sorted last
        let script = report.word_list.script().unwrap_or("\u{200B}Unknown");
        script_exemplars.entry(script).or_default().push(report);
    });
    // Sort reports by name, then by location
    script_exemplars.values_mut().for_each(|reports| {
        reports.sort_unstable_by(|report_a, report_b| {
            Ord::cmp(report_a.word_list.name(), report_b.word_list.name())
                .then_with(|| {
                    PartialOrd::partial_cmp(
                        &report_a.location,
                        &report_b.location,
                    )
                    .unwrap_or(Ordering::Equal)
                })
        });
    });

    html! {
        (DOCTYPE)
        html {
            head {
                title { "Tall Glyphs" }
                meta charset="utf-8";
                style { (CSS) }
            }
            body {
                h1 { "Tall Glyphs" }
                p {
                    "Lines legend:" br;
                    span style="color: green" {
                        "green: [head.yMax, head.yMin]"
                    } br;
                    span style="color: blue" {
                        "blue: [os2.usWinAscent, -os2.usWinDescent]"
                    } br;
                    span style="color: red" {
                        "red: [os2.sTypoAscender, os2.sTypoDescender] "
                        "= clipping limit for Android"
                    } br;
                    span style="color: pink" {
                        (PreEscaped("pink: [1900&frasl;2048&times;upem, "))
                        (PreEscaped("&minus;500&frasl;2048&times;upem]"))
                    } br;
                    span style="color: cyan" {
                        "cyan: BASE table entry for script (if present)"
                    } br;
                }
                @for (script, reports) in script_exemplars {
                    (format_script_reports(script, &reports, font))
                }
            }
        }
    }
    .into_string()
}

struct VerticalFlipPen<'p, P> {
    height: f32,
    inner: &'p mut P,
}

impl<P> OutlinePen for VerticalFlipPen<'_, P>
where
    P: OutlinePen,
{
    fn move_to(&mut self, x: f32, y: f32) {
        debug_assert!(y <= self.height);
        self.inner.move_to(x, self.height - y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        debug_assert!(y <= self.height);
        self.inner.line_to(x, self.height - y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        debug_assert!(y <= self.height);
        self.inner
            .quad_to(cx0, self.height - cy0, x, self.height - y);
    }

    fn curve_to(
        &mut self,
        cx0: f32,
        cy0: f32,
        cx1: f32,
        cy1: f32,
        x: f32,
        y: f32,
    ) {
        debug_assert!(y <= self.height);
        self.inner.curve_to(
            cx0,
            self.height - cy0,
            cx1,
            self.height - cy1,
            x,
            self.height - y,
        );
    }

    fn close(&mut self) {
        self.inner.close()
    }
}

struct OffsetPen<'p, P> {
    offset_x: f32,
    offset_y: f32,
    inner: &'p mut P,
}

impl<P> OutlinePen for OffsetPen<'_, P>
where
    P: OutlinePen,
{
    fn move_to(&mut self, x: f32, y: f32) {
        self.inner.move_to(x + self.offset_x, y + self.offset_y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.inner.line_to(x + self.offset_x, y + self.offset_y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.inner.quad_to(
            cx0 + self.offset_x,
            cy0 + self.offset_y,
            x + self.offset_x,
            y + self.offset_y,
        );
    }

    fn curve_to(
        &mut self,
        cx0: f32,
        cy0: f32,
        cx1: f32,
        cy1: f32,
        x: f32,
        y: f32,
    ) {
        self.inner.curve_to(
            cx0 + self.offset_x,
            cy0 + self.offset_y,
            cx1 + self.offset_x,
            cy1 + self.offset_y,
            x + self.offset_x,
            y + self.offset_y,
        );
    }

    fn close(&mut self) {
        self.inner.close();
    }
}

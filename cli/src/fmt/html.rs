use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, hash_map::Entry},
    fmt,
    fmt::Write,
    ops::Neg,
    rc::Rc,
};

use anyhow::{Context, bail};
use fontheight::{Location, Report, VerticalExtremes};
use harfrust::{ShaperData, ShaperInstance, UnicodeBuffer};
use harfshapedfa::{
    HarfRustShaperExt, ShapingMeta,
    pens::BoundsPen,
    utils::{iso639_to_opentype, iso15924_to_opentype},
};
use log::{debug, error};
use maud::{DOCTYPE, Escaper, Markup, PreEscaped, Render, html};
use ordered_float::NotNan;
use skrifa::{
    FontRef, GlyphId, MetadataProvider, OutlineGlyph,
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
    height: 150px;
    border: 1px grey dashed;
}";

struct RenderUsingDebug<T: fmt::Debug>(T);

impl<T: fmt::Debug> Render for RenderUsingDebug<T> {
    fn render_to(&self, output: &mut String) {
        let mut escaper = Escaper::new(output);
        write!(escaper, "{:?}", self.0).unwrap();
    }
}

#[derive(Debug, Copy, Clone)]
struct SimpleBase {
    min: Option<i16>,
    max: Option<i16>,
}

impl SimpleBase {
    fn line_iter(self) -> impl Iterator<Item = (NotNan<f32>, &'static str)> {
        self.min
            .into_iter()
            .chain(self.max)
            .map(|val| (NotNan::from(val), "cyan"))
    }
}

#[derive(Debug)]
struct LocationCache {
    skrifa_location: skrifa::instance::Location,
    shaper_instance: ShaperInstance,
    glyph_bounds: HashMap<GlyphId, VerticalExtremes>,
    buffer: Option<UnicodeBuffer>,
}

impl LocationCache {
    fn new(font: &FontRef, location: &Location) -> Self {
        Self {
            skrifa_location: location.to_skrifa(font),
            shaper_instance: ShaperInstance::from_variations(
                font,
                location.to_harfrust(),
            ),
            glyph_bounds: Default::default(),
            buffer: Some(UnicodeBuffer::new()),
        }
    }

    fn get_extremes(&mut self, glyph: &OutlineGlyph) -> VerticalExtremes {
        *self
            .glyph_bounds
            .entry(glyph.glyph_id())
            .or_insert_with(|| {
                let mut bounds_pen = BoundsPen::new();
                glyph
                    .draw(
                        DrawSettings::unhinted(
                            Size::unscaled(),
                            &self.skrifa_location,
                        ),
                        &mut bounds_pen,
                    )
                    .unwrap();
                let harfshapedfa::pens::Rect { y0, y1, .. } =
                    bounds_pen.bounding_box();
                VerticalExtremes::new(y0, y1)
            })
    }
}

struct FontCache<'a> {
    font: &'a FontRef<'a>,
    shaper_data: ShaperData,
    //                    (script , language       )
    base_entries: HashMap<(&'a str, Option<&'a str>), Option<SimpleBase>>,
    //                 (y  , colour      )
    const_metrics: Vec<(NotNan<f32>, &'static str)>,
    initial_highest: NotNan<f32>,
    initial_lowest: NotNan<f32>,
}

impl<'a> FontCache<'a> {
    fn new(font: &'a FontRef<'a>) -> anyhow::Result<Self> {
        let os2 = font.os2().context("failed to read OS/2")?;
        let head = font.head().context("failed to read HEAD")?;
        let upm = NotNan::<f32>::from(head.units_per_em());

        let const_metrics = vec![
            // Highs
            (os2.s_typo_ascender().into(), "red"),
            (os2.us_win_ascent().into(), "blue"),
            (head.y_max().into(), "green"),
            (NotNan::new(1900. / 2048.).unwrap() * upm, "pink"),
            // Lows
            (os2.s_typo_descender().into(), "red"),
            (NotNan::<f32>::from(os2.us_win_descent()).neg(), "blue"),
            (head.y_min().into(), "green"),
            (NotNan::new(-500. / 2048.).unwrap() * upm, "pink"),
        ];

        let initial_highest = const_metrics
            .iter()
            .copied()
            .map(|(val, _)| val)
            .max()
            .unwrap();
        let initial_lowest = const_metrics
            .iter()
            .copied()
            .map(|(val, _)| val)
            .min()
            .unwrap();

        Ok(Self {
            shaper_data: ShaperData::new(font),
            base_entries: Default::default(),
            font,
            const_metrics,
            initial_highest,
            initial_lowest,
        })
    }

    fn get_base_entry(
        &mut self,
        word_list: &'a WordList,
    ) -> Option<SimpleBase> {
        fn get_uncached_base_entry(
            font: &FontRef,
            script: &str,
            language: Option<&str>,
        ) -> anyhow::Result<Option<SimpleBase>> {
            let base = match font.base() {
                Ok(base) => base,
                Err(skrifa::raw::ReadError::TableIsMissing(_)) => {
                    return Ok(None);
                },
                Err(why) => bail!("failed to read BASE: {why}"),
            };

            debug!(
                "looking up BASE entry for script: {script}, lang: \
                 {language:?}"
            );

            let ot_script = iso15924_to_opentype(script)
                .context("word list's script wasn't a valid tag")?;
            let ot_language = language
                .map(|lang| {
                    iso639_to_opentype(lang).context(
                        "word list language couldn't be converted to an \
                         OpenType language",
                    )
                })
                .transpose()?;

            let Some(horiz_axis) = base.horiz_axis() else {
                debug!("no horizontal BASE entries");
                return Ok(None);
            };
            let base_script_list = horiz_axis?.base_script_list()?;
            let Some(relevant_script_record) = base_script_list
                .base_script_records()
                .iter()
                .find(|record| record.base_script_tag == ot_script)
            else {
                debug!("no BASE entry with script `{ot_script}`");
                return Ok(None);
            };

            let base_script = relevant_script_record
                .base_script(base_script_list.offset_data())?;

            let language_min_max = ot_language
                .and_then(|lang| {
                    base_script
                        .base_lang_sys_records()
                        .iter()
                        .find(|record| record.base_lang_sys_tag == lang)
                })
                .map(|lang_record| {
                    lang_record.min_max(base_script.offset_data())
                })
                .transpose()?;

            let min_max = match language_min_max {
                None => {
                    let Some(default_min_max) = base_script.default_min_max()
                    else {
                        debug!("no default MinMax for `{ot_script}`");
                        return Ok(None);
                    };
                    debug!("found script BASE entry for `{ot_script}`");
                    default_min_max?
                },
                Some(min_max) => {
                    debug!(
                        "found language-specific BASE override for \
                         `{ot_script}`"
                    );
                    min_max
                },
            };

            let min = min_max
                .min_coord()
                .transpose()?
                .map(|base_coord| base_coord.coordinate());
            let max = min_max
                .max_coord()
                .transpose()?
                .map(|base_coord| base_coord.coordinate());

            Ok(Some(SimpleBase { min, max }))
        }

        let script = word_list.script()?;
        let language = word_list.language();

        match self.base_entries.entry((script, language)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let opt_base =
                    get_uncached_base_entry(self.font, script, language)
                        .unwrap_or_else(|why| {
                            // Store None in the case of errors as it's a
                            // reasonable assumption that they'll be consistent,
                            // and we don't need to emit the error multiple
                            // times every time this script/language combo is
                            // looked up
                            error!(
                                "failed to check for BASE entry (script: \
                                 {script}, lang: {language:?}: {why}",
                            );
                            None
                        });
                *entry.insert(opt_base)
            },
        }
    }
}

fn draw_svg<'a>(
    font_cache: Rc<RefCell<FontCache<'a>>>,
    location_cache: Rc<RefCell<LocationCache>>,
    word: &str,
    word_list: &'a WordList,
) -> SVG {
    let mut font_cache = font_cache.borrow_mut();
    let mut location_cache = location_cache.borrow_mut();

    let mut buffer = location_cache
        .buffer
        .take()
        .expect("GlyphBuffer was not returned to location_cache");
    buffer.push_str(word);

    let shaper = font_cache
        .shaper_data
        .shaper(font_cache.font)
        .instance(Some(&location_cache.shaper_instance))
        .build();
    let shaping_meta = word_list
        .script()
        .map(|script| ShapingMeta::new(script, word_list.language(), &shaper))
        .transpose()
        .unwrap(); // TODO: error handling

    // Default features are still included by default
    let glyph_buffer = match &shaping_meta {
        Some(meta) => shaper.shape_with_meta(meta, buffer, &[]),
        None => {
            buffer.guess_segment_properties();
            shaper.shape(buffer, &[])
        },
    };

    let mut highest = font_cache.initial_highest;
    let mut lowest = font_cache.initial_lowest;

    let maybe_base = font_cache.get_base_entry(word_list);
    if let Some(base) = maybe_base {
        if let Some(max) = base.max {
            highest = highest.max(NotNan::from(max));
        }
        if let Some(min) = base.min {
            lowest = lowest.min(NotNan::from(min));
        }
    }

    let outlines = font_cache.font.outline_glyphs();
    let mut svg_pen = SvgPen::new();
    let (x_max, _) = glyph_buffer
        .glyph_infos()
        .iter()
        .zip(glyph_buffer.glyph_positions())
        .fold(
            (0f32, 0f32),
            |(advance_width, advance_height), (glyph_info, position)| {
                let glyph = outlines.get(glyph_info.glyph_id.into()).unwrap();

                // Draw the glyph
                let offset_x = advance_width + position.x_offset as f32;
                let mut flipped_svg_pen = VerticalFlipPen {
                    height: highest,
                    inner: &mut svg_pen,
                };
                let mut flipped_offset_svg_pen = OffsetPen {
                    offset_x,
                    offset_y: position.y_offset as f32,
                    inner: &mut flipped_svg_pen,
                };
                glyph
                    .draw(
                        DrawSettings::unhinted(
                            Size::unscaled(),
                            &location_cache.skrifa_location,
                        ),
                        &mut flipped_offset_svg_pen,
                    )
                    .unwrap();

                // Look at the bounds and update highest/lowest as needed
                let extrema = location_cache.get_extremes(&glyph);
                highest =
                    highest.max(NotNan::new(extrema.highest() as f32).unwrap());
                lowest =
                    lowest.min(NotNan::new(extrema.lowest() as f32).unwrap());

                // Return position for next glyph to be relative to
                (
                    advance_width + position.x_advance as f32,
                    advance_height + position.y_advance as f32,
                )
            },
        );
    location_cache.buffer = Some(glyph_buffer.clear());

    let word_svg = Path::new().set("d", svg_pen.to_string());

    let x_min = 0.;
    let height = highest;

    let line = |y: NotNan<f32>, colour: &str| {
        let y = y.into_inner();
        Line::new()
            .set("x1", x_min)
            .set("y1", height - y)
            .set("x2", x_max)
            .set("y2", height - y)
            .set("stroke-width", 10)
            .set("stroke", colour)
    };

    font_cache
        .const_metrics
        .iter()
        .copied()
        .chain(maybe_base.into_iter().flat_map(|base| base.line_iter()))
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

fn draw_exemplar<'a>(
    font_cache: Rc<RefCell<FontCache<'a>>>,
    location_cache: Rc<RefCell<LocationCache>>,
    exemplar: &str,
    source: &'a WordList,
    location: &Location,
) -> Markup {
    let svg =
        draw_svg(font_cache, location_cache, exemplar, source).to_string();
    html! {
        li {
            figure {
                (PreEscaped(svg))
                figcaption {
                    "\"" (exemplar) "\" (from " (source.name()) ")" br;
                    (RenderUsingDebug(location))
                }
            }
        }
    }
}

fn format_script_reports<'a>(
    font_cache: Rc<RefCell<FontCache<'a>>>,
    script: &str,
    reports: &[&Report<'a>],
) -> Markup {
    html! {
        details open {
            summary { h2 { (script) } }
            @for report in reports {
                @let location_cache =
                    Rc::new(RefCell::new(LocationCache::new(font_cache.borrow().font, report.location)));
                ul.drawn {
                    @for high_exemplar in report.exemplars.highest() {
                        (draw_exemplar(
                            font_cache.clone(),
                            location_cache.clone(),
                            high_exemplar.word,
                            report.word_list,
                            report.location,
                        ))
                    }
                    @for low_exemplar in report.exemplars.lowest() {
                        (draw_exemplar(
                            font_cache.clone(),
                            location_cache.clone(),
                            low_exemplar.word,
                            report.word_list,
                            report.location,
                        ))
                    }
                }
            }
        }
    }
}

pub fn format_all_reports(
    reports: &[Report],
    font: &FontRef,
) -> anyhow::Result<String> {
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
                    .expect("fontheight generated unsortable locations")
                })
        });
    });

    let font_cache = Rc::new(RefCell::new(FontCache::new(font)?));

    let html = html! {
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
                    (format_script_reports(font_cache.clone(), script, &reports))
                }
            }
        }
    };
    Ok(html.into_string())
}

struct VerticalFlipPen<'p, P> {
    height: NotNan<f32>,
    inner: &'p mut P,
}

impl<P> OutlinePen for VerticalFlipPen<'_, P>
where
    P: OutlinePen,
{
    fn move_to(&mut self, x: f32, y: f32) {
        debug_assert!(y <= self.height.into_inner());
        self.inner.move_to(x, self.height - y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        debug_assert!(y <= self.height.into_inner());
        self.inner.line_to(x, self.height - y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        debug_assert!(y <= self.height.into_inner());
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
        debug_assert!(y <= self.height.into_inner());
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

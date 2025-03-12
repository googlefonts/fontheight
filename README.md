# Font Height

`fontheight` is a tool that provides recommendations on setting font vertical metrics based on **shaped words**.

## Motivation

Vertical metrics frequently decide clipping boundaries, but are not used consistently across platforms: e.g. Windows uses OS/2 WinAscent/WinDescent, whereas for system fonts [Android uses TypoAscent/TypoDescent and a combination of custom heuristics](https://simoncozens.github.io/android-clipping/).

It is often desirable to derive metrics from shaped words as opposed to individual glyphs, as words may reach greater extents:

> Early versions of this specification suggested that the usWinAscent value be computed as the yMax for all characters in the Windows “ANSI” character set.
> For new fonts, the value should be determined based on the primary languages the font is designed to support, and should take into consideration additional height that could be required to accommodate tall glyphs or mark positioning.

⬆️ [OS/2 — OS/2 and Windows Metrics Table, OpenType Specification 1.9.1](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#uswinascent)

For this reason, vertical metrics must be chosen with a combination of design (e.g. aesthetic, legibility) and engineering (e.g. clipping) considerations in mind.
For the latter, `fontheight` evaluates the extents of a corpus of shaped text across each writing system that a font intends to support.

## Usage & installation

Fontheight comes in three flavours:
1. A commandline tool (`fontheight`)
2. A basic Python API (`fontheight-wheel`)
3. A Rust library (`fontheight-core`)

### `fontheight` as a commandline tool

#### Installation

From GitHub:

```shell
STATIC_LANG_WORD_LISTS_LOCAL=1 cargo install --locked --git https://github.com/googlefonts/fontheight fontheight
```

⚠️ _Not yet available_
From crates.io:
```shell
cargo install --locked fontheight
```

#### Usage

```
Usage: fontheight [OPTIONS] [FONT_PATH]...

Arguments:
  [FONT_PATH]...  The TTF(s) to analyze

Options:
  -n, --results <RESULTS>       The number of words to log [default: 5]
      --words <WORDS_PER_LIST>  The number of words from each list to test [default: 25]
  -h, --help                    Print help
  -V, --version                 Print version
```

### `fontheight`'s Python API

⚠️ _Not yet available_

The API will be available under the package `libfontheight` and then is imported as `fontheight`

#### Usage

See the method signatures & data types below, approximately written in Python. `k_words` is equivalent to `--words` in the CLI, and `n_exemplars` is equivalent to `-n/--results`

```python
import fontheight

# Entrypoints

fontheight.get_min_max_extremes_from(path: os.pathLike, k_words: int, n_exemplars: int) -> list[fontheight.Report]

fontheight.get_min_max_extremes_from(font_bytes: bytes, k_words: int, n_exemplars: int) -> list[fontheight.Report]

# Returned data types

@dataclass(frozen=True)
class fontheight.Report:
    location: dict[str, float]
    word_list_name: str
    exemplars: fontheight.Exemplars

@dataclass(frozen=True)
class fontheight.Exemplars:
    lowest: list[fontheight.WordExtremes]  # sorted, lowest lows first
    highest: list[fontheight.WordExtremes]  # sorted, highest highs first

@dataclass(frozen=True)
class fontheight.WordExtremes:
    word: str
    lowest: float
    highest: float
```

### `fontheight`'s Rust crate

⚠️ _Not yet available_ On crates.io as `fontheight-core`

For documentation, please refer to docs.rs

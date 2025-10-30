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

Fontheight comes in two flavours:
1. A commandline tool
2. A Rust library

### `fontheight` as a commandline tool

#### Installation

Currently, both installation methods involve compiling `fontheight` from its source code, meaning you need to have a Rust toolchain installed and `cargo` available in your terminal.
If you're new to Rust, check out https://www.rust-lang.org/tools/install for instructions on getting it installed and configured.
You only need a stable compiler for Font Height's crates.

From crates.io (note: the build script for `static-lang-word-lists` requires network access, see [its README](static-lang-word-lists/README.md) for why):
```shell
cargo install --locked fontheight-cli
```

From GitHub:
```shell
STATIC_LANG_WORD_LISTS_LOCAL=1 cargo install --locked --git https://github.com/googlefonts/fontheight fontheight-cli
```

Note: after installing, the command you run is `fontheight` (not `fontheight-cli`).

#### Usage

```
Usage: fontheight [OPTIONS] <FONT_PATH>...

Arguments:
  <FONT_PATH>...  The TTF(s) to analyze

Options:
  -n, --results <RESULTS>       The number of words to log [default: 5]
  -k, --words <WORDS_PER_LIST>  The number of words from each list to test [default: all words]
  -v, --verbose...              Increase logging verbosity
  -q, --quiet...                Decrease logging verbosity
  -o, --output <OUTPUT_PATH>    Write the reports into the given path. Will print to stdout if not specified
      --html                    Output all the reports into a single HTML file
  -h, --help                    Print help
  -V, --version                 Print version
```

Most of the word list shipped with `fontheight` are sorted by greatest vertical extremes to try and help reduce the number of words which need to be checked to produce a useful report, should you not wish to test the full word lists (which may be time consuming).

<details>
<summary>Unsorted word lists</summary>

- DiffenatorBopomofo
- DiffenatorGeorgian
- DiffenatorHiragana
- DiffenatorJapanese
- DiffenatorKatakana
- DiffenatorThanaa
- DiffenatorTifinagh

</details>

<details>
<summary>Sorted word lists (and fonts used in sorting)</summary>

Sorted DiffenatorAdlam based on:
- NotoSansAdlam[wght].ttf
- NotoSansAdlamUnjoined[wght].ttf

Sorted DiffenatorArabic based on:
- NotoKufiArabic[wght].ttf
- NotoNaskhArabic[wght].ttf
- NotoSansArabic[wdth,wght].ttf

Sorted DiffenatorArmenian based on:
- NotoSansArmenian[wdth,wght].ttf
- NotoSerifArmenian[wdth,wght].ttf

Sorted DiffenatorAvestan based on:
- NotoSansAvestan-Regular.ttf

Sorted DiffenatorBengali based on:
- NotoSansBengali[wdth,wght].ttf
- NotoSerifBengali[wdth,wght].ttf

Sorted DiffenatorCanadian_Aboriginal based on:
- NotoSansCanadianAboriginal[wght].ttf

Sorted DiffenatorChakma based on:
- NotoSansChakma-Regular.ttf

Sorted DiffenatorCherokee based on:
- NotoSansCherokee[wght].ttf

Sorted DiffenatorCommon based on:
  - NotoSansLGC[wdth,wght].ttf
  - NotoSansMonoLGC[wdth,wght].ttf
  - NotoSerifLGC[wdth,wght].ttf

Sorted DiffenatorCyrillic based on:
- NotoSansLGC[wdth,wght].ttf
- NotoSansMonoLGC[wdth,wght].ttf
- NotoSerifLGC[wdth,wght].ttf

Sorted DiffenatorDevanagari based on:
- NotoSansDevanagari[wdth,wght].ttf
- NotoSerifDevanagari[wdth,wght].ttf

Sorted DiffenatorEthiopic based on:
- NotoSansEthiopic[wdth,wght].ttf
- NotoSerifEthiopic[wdth,wght].ttf

Sorted DiffenatorGreek based on:
- NotoSansLGC[wdth,wght].ttf
- NotoSansMonoLGC[wdth,wght].ttf
- NotoSerifLGC[wdth,wght].ttf

Sorted DiffenatorGujarati based on:
- NotoSansGujarati[wdth,wght].ttf
- NotoSerifGujarati[wght].ttf

Sorted DiffenatorGurmukhi based on:
- NotoSansGurmukhi[wdth,wght].ttf
- NotoSerifGurmukhi[wght].ttf

Sorted DiffenatorHebrew based on:
- NotoRashiHebrew[wght].ttf
- NotoSansHebrew[wdth,wght].ttf
- NotoSerifHebrew[wdth,wght].ttf

Sorted DiffenatorKhmer based on:
- NotoSansKhmer[wdth,wght].ttf
- NotoSerifKhmer[wdth,wght].ttf

Sorted DiffenatorLao based on:
- NotoSansLao[wdth,wght].ttf
- NotoSansLaoLooped[wdth,wght].ttf
- NotoSerifLao[wdth,wght].ttf

Sorted DiffenatorLatin based on:
- NotoSansLGC[wdth,wght].ttf
- NotoSansMonoLGC[wdth,wght].ttf
- NotoSerifLGC[wdth,wght].ttf

Sorted DiffenatorLisu based on:
- NotoSansLisu[wght].ttf

Sorted DiffenatorMalayalam based on:
- NotoSansMalayalam[wdth,wght].ttf
- NotoSerifMalayalam[wght].ttf

Sorted DiffenatorMongolian based on:
- NotoSansMongolian-Regular.ttf

Sorted DiffenatorMyanmar based on:
- NotoSansMyanmar[wdth,wght].ttf
- NotoSerifMyanmar[wdth,wght].ttf

Sorted DiffenatorOl_Chiki based on:
- NotoSansOlChiki[wght].ttf

Sorted DiffenatorOriya based on:
- NotoSansOriya[wdth,wght].ttf
- NotoSerifOriya[wght].ttf

Sorted DiffenatorOsage based on:
- NotoSansOsage-Regular.ttf

Sorted DiffenatorSinhala based on:
- NotoSansSinhala[wdth,wght].ttf
- NotoSerifSinhala[wdth,wght].ttf

Sorted DiffenatorSyriac based on:
- NotoSansSyriac[wght].ttf
- NotoSansSyriacEastern[wght].ttf
- NotoSansSyriacWestern[wght].ttf

Sorted DiffenatorTamil based on:
- NotoSansTamil[wdth,wght].ttf
- NotoSerifTamil[wdth,wght].ttf

Sorted DiffenatorTelugu based on:
- NotoSansTelugu[wdth,wght].ttf
- NotoSerifTelugu[wght].ttf

Sorted DiffenatorThai based on:
- NotoSansThai[wdth,wght].ttf
- NotoSansThaiLooped[wdth,wght].ttf
- NotoSerifThai[wdth,wght].ttf

Sorted DiffenatorTibetan based on:
- NotoSerifTibetan[wght].ttf

Sorted DiffenatorVai based on:
- NotoSansVai-Regular.ttf

</details>

### `fontheight`'s Rust crate

On crates.io as `fontheight`.

For documentation, please refer to [docs.rs](https://docs.rs/fontheight/latest).

### `static-lang-word-lists` Rust crate

Provides word lists embedded within the application binary that are compressed at build time, and lazily decompressed at runtime.

See the crate [README](static-lang-word-lists/README.md) & [documentation](https://docs.rs/static-lang-word-lists) for what's available.

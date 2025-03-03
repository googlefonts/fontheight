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

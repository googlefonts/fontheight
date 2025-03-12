from pathlib import Path

import fontheight

FONT_DIR = Path(__file__).parent.parent / "fonts"
WORD_LIST_DIR = Path(__file__).parent.parent / "data"

JOBS: list[tuple[str, str]] = [
    ("diffenator/Adlam.txt", "*Adlam*.ttf"),
    ("diffenator/Arabic.txt", "*Arabic*.ttf"),
    ("diffenator/Armenian.txt", "*Armenian*.ttf"),
    ("diffenator/Avestan.txt", "*Avestan*.ttf"),
    ("diffenator/Bengali.txt", "*Bengali*.ttf"),
    ("diffenator/Canadian_Aboriginal.txt", "*CanadianAboriginal*.ttf"),
    ("diffenator/Chakma.txt", "*Chakma*.ttf"),
    ("diffenator/Cherokee.txt", "*Cherokee*.ttf"),
    ("diffenator/Common.txt", "*LGC*.ttf"),
    ("diffenator/Cyrillic.txt", "*LGC*.ttf"),
    ("diffenator/Devanagari.txt", "*Devanagari*.ttf"),
    ("diffenator/Ethiopic.txt", "*Ethiopic*.ttf"),
    ("diffenator/Greek.txt", "*LGC*.ttf"),
    ("diffenator/Gujarati.txt", "*Gujarati*.ttf"),
    ("diffenator/Gurmukhi.txt", "*Gurmukhi*.ttf"),
    ("diffenator/Hebrew.txt", "*Hebrew*.ttf"),
    ("diffenator/Khmer.txt", "*Khmer*.ttf"),
    ("diffenator/Lao.txt", "*Lao*.ttf"),
    ("diffenator/Latin.txt", "*LGC*.ttf"),
    ("diffenator/Lisu.txt", "*Lisu*.ttf"),
    ("diffenator/Malayalam.txt", "*Malayalam*.ttf"),
    ("diffenator/Mongolian.txt", "*Mongolian*.ttf"),
    ("diffenator/Myanmar.txt", "*Myanmar*.ttf"),
    ("diffenator/Ol_Chiki.txt", "*OlChiki*.ttf"),
    ("diffenator/Oriya.txt", "*Oriya*.ttf"),
    ("diffenator/Osage.txt", "*Osage*.ttf"),
    ("diffenator/Sinhala.txt", "*Sinhala*.ttf"),
    ("diffenator/Syriac.txt", "*Syriac*.ttf"),
    ("diffenator/Tamil.txt", "*Tamil*.ttf"),
    ("diffenator/Telugu.txt", "*Telugu*.ttf"),
    ("diffenator/Thai.txt", "*Thai*.ttf"),
    ("diffenator/Tibetan.txt", "*Tibetan*.ttf"),
    ("diffenator/Vai.txt", "*Vai*.ttf"),
]


# Copied from egg.py
def word_list_name(path: Path) -> str:
    return "".join(
        part.title() for part in path.relative_to(WORD_LIST_DIR).with_suffix("").parts
    )


def main():
    scripts_sorted: set[str] = set()

    for word_list_rel_path, glob in JOBS:
        word_list_path = WORD_LIST_DIR / word_list_rel_path
        current_word_list_name = word_list_name(word_list_path)

        # Ordered set!
        original_word_list = dict.fromkeys(
            word_list_path.read_text(encoding="utf-8").splitlines()
        )

        # Mapping of words to their desired index in the sorted list
        word_priorities: dict[str, int] = dict()

        font_paths = sorted(FONT_DIR.glob(glob))
        assert len(font_paths) > 0, f'no fonts matched "{glob}"'
        for font in font_paths:
            results = fontheight._get_all_word_list_extremes(font, current_word_list_name)

            # Highest pass
            for index, word_extreme in enumerate(
                sorted(results, key=lambda we: we.highest, reverse=True)
            ):
                new_value = (
                    index
                    if (current_index := word_priorities.get(word_extreme.word)) is None
                    else min(current_index, index)
                )
                word_priorities[word_extreme.word] = new_value

            # Lowest pass
            for index, word_extreme in enumerate(
                sorted(results, key=lambda we: we.lowest)
            ):
                new_value = (
                    index
                    if (current_index := word_priorities.get(word_extreme.word)) is None
                    else min(current_index, index)
                )
                word_priorities[word_extreme.word] = new_value

        # Produce the new sorted list
        ordered = [
            word
            for _, word in sorted(
                (index, word) for word, index in word_priorities.items()
            )
        ]

        for word in ordered:
            del original_word_list[word]
        ordered.extend(original_word_list.keys())

        word_list_path.write_text("\n".join(ordered) + "\n", encoding="utf-8")
        scripts_sorted.add(word_list_path.stem)
        print(
            f"Sorted {current_word_list_name} based on:\n  -",
            "\n  - ".join(font_path.name for font_path in font_paths),
        )
        print()

    all_scripts = set(
        word_list_path.stem
        for word_list_path in (WORD_LIST_DIR / "diffenator").glob("*.txt")
    )
    unchanged_scripts = sorted(all_scripts - scripts_sorted)
    if len(unchanged_scripts) > 0:
        # TODO: make this output the word list name as exported by the crate
        print(
            "Unsorted scripts:\n  -",
            "\n  - ".join(unchanged_scripts),
        )


if __name__ == "__main__":
    main()

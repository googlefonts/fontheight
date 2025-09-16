#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.9"
# dependencies = [
#     "python-iso639",
#     "tomli-w",
# ]
# ///

from pathlib import Path

import tomli_w
from iso639 import Language, LanguageNotFoundError

AOSP_DIR = Path(__file__).parent.parent / "data" / "aosp"

for word_list_path in AOSP_DIR.glob("*.txt"):
    components = word_list_path.stem.split("_")  # lang_script
    language_tag = components[0]
    script = components[1] if len(components) == 2 else None

    try:
        language = Language.from_part1(language_tag)
    except LanguageNotFoundError:
        print(
            f"removed {word_list_path.relative_to(AOSP_DIR.parent)}, language not recognised"
        )
        word_list_path.unlink()
        continue
    name_clean = language.name.lower().replace(" (macrolanguage)", "").replace(" ", "_")
    doc = {
        "name": f"aosp_{name_clean}" + (f"_{script}" if script else ""),
        "language": language_tag,
    }
    if script:
        doc["script"] = script
    metadata_path = word_list_path.with_suffix(".toml")
    metadata_path.write_text(tomli_w.dumps(doc), encoding="utf-8")
    print(f"wrote {metadata_path.relative_to(AOSP_DIR.parent)}")

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
    try:
        language = Language.from_part1(word_list_path.stem)
    except LanguageNotFoundError:
        print(
            f"removed {word_list_path.relative_to(AOSP_DIR.parent)}, language not recognised"
        )
        word_list_path.unlink()
        continue
    name_clean = language.name.lower().replace(" (macrolanguage)", "").replace(" ", "_")
    doc = {
        "name": f"aosp_{name_clean}",
        "language": word_list_path.stem,
    }
    metadata_path = word_list_path.with_suffix(".toml")
    metadata_path.write_text(tomli_w.dumps(doc), encoding="utf-8")
    print(f"wrote {metadata_path.relative_to(AOSP_DIR.parent)}")

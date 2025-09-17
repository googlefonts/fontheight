#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.9"
# dependencies = [
#     "babelfish>=0.6",
#     "tomli-w",
# ]
# ///

from pathlib import Path

import tomli_w
from babelfish import Language, LanguageReverseError, Script

AOSP_DIR = Path(__file__).parent.parent / "data" / "aosp"

for word_list_path in AOSP_DIR.glob("*.txt"):
    components = word_list_path.stem.split("_")  # lang_script
    language_tag = components[0]
    script = Script(components[1]) if len(components) == 2 else None

    try:
        language = Language.fromalpha2(language_tag)
    except LanguageReverseError:
        print(
            f"removed {word_list_path.relative_to(AOSP_DIR.parent)}, language not recognised"
        )
        word_list_path.unlink()
        continue

    name_clean = (
        language.name.lower()
        .replace(" (macrolanguage)", "")
        # Edge case for el (modern Greek)
        .replace(" (1453-)", "")
        .replace(" ", "_")
    )
    script_name_clean = (
        "_" + script.name.lower().split(" ")[0] if script is not None else ""
    )
    # Avoid repeating script name if it's within the language name
    if script_name_clean in "_" + name_clean:
        script_name_clean = ""

    doc = {
        "name": f"aosp_{name_clean}{script_name_clean}",
        "language": language_tag,
    }
    if script:
        doc["script"] = script.code
    metadata_path = word_list_path.with_suffix(".toml")
    metadata_path.write_text(tomli_w.dumps(doc), encoding="utf-8")
    print(f"wrote {metadata_path.relative_to(AOSP_DIR.parent)}")

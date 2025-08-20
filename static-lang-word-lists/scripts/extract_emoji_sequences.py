"""Generate an exhaustive "user word list" for Diffenator.

The data has been taken from

- https://www.unicode.org/Public/draft/emoji/emoji-test.txt
- https://www.unicode.org/Public/draft/ucd/emoji/emoji-variation-sequences.txt

to get the newest working draft data.

This generates three lists:

- diffenator_emoji_all: All entries of emoji-test.txt plus all entries of
  emoji-variation-sequences.txt
- diffenator_emoji_color: All entries of emoji-test.txt plus all color (VS16)
  variants of emoji-variation-sequences.txt
- diffenator_emoji_textual: All entries of emoji-test.txt without explicit VS16
  character plus all textual (VS15) variants of emoji-variation-sequences.txt
"""

from pathlib import Path

# U+FE0F: VARIATION SELECTOR-16, for selecting color variants of emojis.
VS16 = chr(0xFE0F)

sequences_all: list[list[str]] = []
sequences_color: list[list[str]] = []
sequences_textual: list[list[str]] = []


def new_subgroups():
    sequences_all.append([])
    sequences_color.append([])
    sequences_textual.append([])


# 1. Use the official test file to extract all official sequences to be
#    supported.
for line in Path("data/third_party/ucd/emoji-test.txt").read_text().splitlines():
    line = line.strip()
    if line.startswith("# subgroup"):
        new_subgroups()
    if not line or line.startswith("#"):
        continue
    sequence, *_ = line.split(";")
    sequence = sequence.strip()
    codepoints = [chr(int(v, 16)) for v in sequence.split(" ")]
    codepoints_string = "".join(codepoints)
    sequences_all[-1].append(codepoints_string)
    sequences_color[-1].append(codepoints_string)
    if VS16 not in codepoints:
        sequences_textual[-1].append(codepoints_string)
new_subgroups()

# 2. Use the official variation sequences file to extract all explicit textual
#    or color representation sequences to be supported.
emoji_variation_sequences = Path("data/third_party/ucd/emoji-variation-sequences.txt")
for line in emoji_variation_sequences.read_text().splitlines():
    line = line.strip()
    if not line or line.startswith("#"):
        continue
    sequence, *_ = line.split(";")
    sequence = sequence.strip()
    codepoints = [chr(int(v, 16)) for v in sequence.split(" ")]
    codepoints_string = "".join(codepoints)
    if VS16 in codepoints:
        sequences_all[-1].append(codepoints_string)
        sequences_color[-1].append(codepoints_string)
    else:
        sequences_all[-1].append(codepoints_string)
        sequences_textual[-1].append(codepoints_string)
new_subgroups()

Path("data/diffenator/Emoji_All.toml").write_text("""\
name = "diffenator_emoji_all"
script = "Zyyy"
""")
emoji_all = "\n".join(" ".join(subgroup) for subgroup in sequences_all)
Path("data/diffenator/Emoji_All.txt").write_text(emoji_all)

Path("data/diffenator/Emoji_Color.toml").write_text("""\
name = "diffenator_emoji_color"
script = "Zyyy"
""")
emoji_color = "\n".join(" ".join(subgroup) for subgroup in sequences_color)
Path("data/diffenator/Emoji_Color.txt").write_text(emoji_color)

Path("data/diffenator/Emoji_Textual.toml").write_text("""\
name = "diffenator_emoji_textual"
script = "Zyyy"
""")
emoji_textual = "\n".join(" ".join(subgroup) for subgroup in sequences_textual)
Path("data/diffenator/Emoji_Textual.txt").write_text(emoji_textual)

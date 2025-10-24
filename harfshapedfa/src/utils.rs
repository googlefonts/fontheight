use harfrust::{Direction, Script, Tag, script};

use crate::errors::InvalidTagError;

#[must_use]
pub const fn direction_from_script(script: Script) -> Option<Direction> {
    // Copied from harfrust (internal API)
    // https://github.com/harfbuzz/harfrust/blob/bf4b7ca20cf95e7183c5f9e1c13a56e9ca6c1174/src/hb/common.rs#L75-L161

    match script {
        // Unicode-1.1 additions
        script::ARABIC |
        script::HEBREW |

        // Unicode-3.0 additions
        script::SYRIAC |
        script::THAANA |

        // Unicode-4.0 additions
        script::CYPRIOT |

        // Unicode-4.1 additions
        script::KHAROSHTHI |

        // Unicode-5.0 additions
        script::PHOENICIAN |
        script::NKO |

        // Unicode-5.1 additions
        script::LYDIAN |

        // Unicode-5.2 additions
        script::AVESTAN |
        script::IMPERIAL_ARAMAIC |
        script::INSCRIPTIONAL_PAHLAVI |
        script::INSCRIPTIONAL_PARTHIAN |
        script::OLD_SOUTH_ARABIAN |
        script::OLD_TURKIC |
        script::SAMARITAN |

        // Unicode-6.0 additions
        script::MANDAIC |

        // Unicode-6.1 additions
        script::MEROITIC_CURSIVE |
        script::MEROITIC_HIEROGLYPHS |

        // Unicode-7.0 additions
        script::MANICHAEAN |
        script::MENDE_KIKAKUI |
        script::NABATAEAN |
        script::OLD_NORTH_ARABIAN |
        script::PALMYRENE |
        script::PSALTER_PAHLAVI |

        // Unicode-8.0 additions
        script::HATRAN |

        // Unicode-9.0 additions
        script::ADLAM |

        // Unicode-11.0 additions
        script::HANIFI_ROHINGYA |
        script::OLD_SOGDIAN |
        script::SOGDIAN |

        // Unicode-12.0 additions
        script::ELYMAIC |

        // Unicode-13.0 additions
        script::CHORASMIAN |
        script::YEZIDI |

        // Unicode-14.0 additions
        script::OLD_UYGHUR => {
            Some(Direction::RightToLeft)
        }

        // https://github.com/harfbuzz/harfbuzz/issues/1000
        script::OLD_HUNGARIAN |
        script::OLD_ITALIC |
        script::RUNIC |
        script::TIFINAGH => {
            None
        }

        _ => Some(Direction::LeftToRight),
    }
}

// https://github.com/simoncozens/autobase/blob/9887854fd7436d034c15bf5875686b7583536e76/autobase/src/utils.rs#L223-L248
pub fn iso15924_to_opentype(script: &str) -> Result<Tag, InvalidTagError> {
    match script {
        // Special cases: https://github.com/fonttools/fonttools/blob/3c1822544d608f87c41fc8fb9ba41ea129257aa8/Lib/fontTools/unicodedata/OTTags.py
        // Relevant specification: https://learn.microsoft.com/en-us/typography/opentype/spec/scripttags
        // SCRIPT_EXCEPTIONS
        "Hira" => Ok(Tag::new(b"kana")),
        "Hrkt" => Ok(Tag::new(b"kana")),
        "Laoo" => Ok(Tag::new(b"lao ")),
        "Yiii" => Ok(Tag::new(b"yi  ")),
        "Nkoo" => Ok(Tag::new(b"nko ")),
        "Vaii" => Ok(Tag::new(b"vai ")),
        // NEW_SCRIPT_TAGS
        "Beng" => Ok(Tag::new(b"bng2")),
        "Deva" => Ok(Tag::new(b"dev2")),
        "Gujr" => Ok(Tag::new(b"gjr2")),
        "Guru" => Ok(Tag::new(b"gur2")),
        "Knda" => Ok(Tag::new(b"knd2")),
        "Mlym" => Ok(Tag::new(b"mlm2")),
        "Orya" => Ok(Tag::new(b"ory2")),
        "Taml" => Ok(Tag::new(b"tml2")),
        "Telu" => Ok(Tag::new(b"tel2")),
        "Mymr" => Ok(Tag::new(b"mym2")),
        // General case
        _ => Tag::new_checked(script.to_lowercase().as_bytes())
            .map_err(InvalidTagError),
    }
}

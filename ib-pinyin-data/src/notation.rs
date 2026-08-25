use super::{Pinyin, PinyinString};

bitflags::bitflags! {
    /// - All pinyin notations are in lower case (`py.to_lowercase() == py`).
    /// - All pinyin notations are no more than 7 characters long (`py.len() <= 7`).
    ///
    /// ## Others
    /// TODO: doc alias does not work
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct PinyinNotation: u32 {
        /// e.g. "pīn", "yīn"
        const Unicode = 0x8;

        /// 全拼
        ///
        /// e.g. "pin", "yin"
        ///
        /// See [全拼](https://zh.wikipedia.org/wiki/全拼) for details.
        #[doc(alias = "全拼")]
        const Ascii = 0x2;

        /// 带声调全拼
        ///
        /// The tone digit is in `1..=5`. See [tones](https://en.wikipedia.org/wiki/Pinyin#Tones) for details.
        ///
        /// e.g. "pin1", "yin1"
        #[doc(alias = "带声调全拼")]
        const AsciiTone = 0x4;

        /// 简拼
        ///
        /// e.g. "p", "y"
        ///
        /// See [简拼](https://zh.wikipedia.org/wiki/简拼) for details.
        #[doc(alias = "简拼")]
        const AsciiFirstLetter = 0x1;

        /// 智能 ABC 双拼
        ///
        /// See [智能ABC输入法](https://zh.wikipedia.org/wiki/智能ABC输入法#双拼方案) for details.
        #[doc(alias = "智能ABC双拼")]
        const DiletterAbc = 0x10;

        /// 拼音加加双拼
        ///
        /// See [拼音加加](https://zh.wikipedia.org/wiki/拼音加加#双拼方案) for details.
        #[doc(alias = "拼音加加双拼")]
        const DiletterJiajia = 0x20;

        /// 微软双拼
        ///
        /// See [微软拼音输入法](https://zh.wikipedia.org/wiki/微软拼音输入法#双拼方案) for details.
        #[doc(alias = "微软双拼")]
        const DiletterMicrosoft = 0x40;

        /// 华宇双拼（紫光双拼）
        ///
        /// See [华宇拼音输入法](https://zh.wikipedia.org/wiki/华宇拼音输入法#双拼方案) for details.
        #[doc(alias("华宇双拼", "紫光双拼"))]
        const DiletterThunisoft = 0x80;

        /// 小鹤双拼
        ///
        /// See [小鹤双拼](https://flypy.com/) for details.
        #[doc(alias = "小鹤双拼")]
        const DiletterXiaohe = 0x100;

        /// 自然码双拼
        ///
        /// See [自然码](https://zh.wikipedia.org/zh-cn/自然码) for details.
        #[doc(alias = "自然码双拼")]
        const DiletterZrm = 0x200;
    }
}

impl PinyinNotation {
    pub fn contains_diletter(&self) -> bool {
        self.intersects(
            PinyinNotation::DiletterAbc
                | PinyinNotation::DiletterJiajia
                | PinyinNotation::DiletterMicrosoft
                | PinyinNotation::DiletterThunisoft
                | PinyinNotation::DiletterXiaohe
                | PinyinNotation::DiletterZrm,
        )
    }

    /// `None` if no notation is set.
    pub fn max_len(&self) -> Option<usize> {
        if self.intersects(PinyinNotation::Unicode | PinyinNotation::AsciiTone) {
            return Some(7);
        }
        if self.contains(PinyinNotation::Ascii) {
            return Some(6);
        }
        if self.contains_diletter() {
            return Some(2);
        }
        if self.contains(PinyinNotation::AsciiFirstLetter) {
            return Some(1);
        }
        None
    }
}

#[cfg(feature = "inmut-data")]
pub(super) use atomic::*;
#[cfg(feature = "inmut-data")]
mod atomic {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    pub struct AtomicPinyinNotation(AtomicU32);

    impl Clone for AtomicPinyinNotation {
        fn clone(&self) -> Self {
            Self(self.0.load(Ordering::Relaxed).into())
        }
    }

    impl From<PinyinNotation> for AtomicPinyinNotation {
        fn from(notation: PinyinNotation) -> Self {
            Self(notation.bits().into())
        }
    }

    impl Into<PinyinNotation> for AtomicPinyinNotation {
        fn into(self) -> PinyinNotation {
            PinyinNotation::from_bits_retain(self.0.load(Ordering::Relaxed))
        }
    }

    impl AtomicPinyinNotation {
        pub fn bitor_assign(&self, rhs: PinyinNotation) {
            self.0.fetch_or(rhs.bits(), Ordering::Relaxed);
        }
    }
}

pub(super) fn unicode_to_ascii(unicode: &str) -> PinyinString {
    let mut ascii = PinyinString::new();
    let mut chars = unicode.chars();
    while let Some(c) = chars.next() {
        match c {
            'a'..='z' => {
                ascii.extend([c]);
                let mut chars_try_next = chars.clone();
                if let Some(c) = chars_try_next.next() {
                    if c == '̀' {
                        chars = chars_try_next;
                    }
                }
            }
            _ => ascii.extend([match c {
                'ā' | 'á' | 'ǎ' | 'à' => b'a',
                'ē' | 'é' | 'ě' | 'è' | 'ế' | 'ề' => b'e',
                // "ê̄" | "ê̌" => b'e',
                'ê' => {
                    let c = chars.next();
                    assert!(c == Some('̄') || c == Some('̌'));
                    b'e'
                }
                'ī' | 'í' | 'ǐ' | 'ì' => b'i',
                'ō' | 'ó' | 'ǒ' | 'ò' => b'o',
                'ū' | 'ú' | 'ǔ' | 'ù' => b'u',
                'ü' | 'ǘ' | 'ǚ' | 'ǜ' => b'v',
                'ń' | 'ň' | 'ǹ' => b'n',
                'ḿ' => b'm',
                // "m̀" begins with 'm'
                // "m̀" => b'm',
                _ => unreachable!(),
            } as char]),
        }
    }
    ascii
}

pub(super) fn unicode_to_ascii_tone(unicode: &str) -> PinyinString {
    let mut ascii = unicode_to_ascii(unicode);
    let tone = match unicode_tone(unicode) {
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        _ => unreachable!(),
    };
    unsafe { ascii.push_unchecked(tone) }
    ascii
}

fn unicode_tone(unicode: &str) -> u8 {
    for c in unicode.chars() {
        match c {
            'ā' | 'ē' | 'ī' | 'ō' | 'ū' |
            // 'ê̄'
            '̄' => return 1,
            'á' | 'é' | 'ế' | 'í' | 'ó' | 'ú' | 'ǘ' | 'ḿ' | 'ń' => return 2,
            'ǎ' | 'ě' | 'ǐ' | 'ǒ' | 'ǔ' | 'ǚ' | 'ň' |
            // 'ê̌'
            '̌' => return 3,
            'à' | 'è' | 'ề' | 'ì' | 'ò' | 'ù' | 'ǜ' | 'ǹ' |
            // 'm̀'
            '̀' => return 4,
            _ => (),
        }
    }
    5
}

pub(super) fn ascii_map_fn(notation: PinyinNotation) -> fn(&str) -> PinyinString {
    match notation {
        PinyinNotation::DiletterAbc => ascii_to_diletter_abc,
        PinyinNotation::DiletterJiajia => ascii_to_diletter_jiajia,
        PinyinNotation::DiletterMicrosoft => ascii_to_diletter_microsoft,
        PinyinNotation::DiletterThunisoft => ascii_to_diletter_thunisoft,
        PinyinNotation::DiletterXiaohe => ascii_to_diletter_xiaohe,
        PinyinNotation::DiletterZrm => ascii_to_diletter_zrm,
        _ => unreachable!(),
    }
}

/// ## Arguments
/// - `map_initial`
///
///   At least "zh", "ch" and "sh" must be mapped.
///
///   See [initials](https://en.wikipedia.org/wiki/Pinyin#Initials) for details.
///
/// - `final_map`: See [finals](https://en.wikipedia.org/wiki/Pinyin#Finals) for details.
fn ascii_to_diletter<'a>(
    ascii: &str,
    map_pinyin: impl Fn(&str) -> Option<&str>,
    map_initial: impl Fn(&str) -> Option<&str>,
    map_final: impl Fn(&str) -> Option<&str>,
) -> PinyinString {
    let ascii = match ascii {
        // 噷
        "hm" => "hen",
        // 哼
        "hng" => "heng",
        // 呒呣嘸
        "m" => "mu",
        // 唔嗯 㕶 𠮾
        "n" | "ng" => "en",
        _ => ascii,
    };

    if let Some(py) = map_pinyin(ascii) {
        return py.into();
    }

    match Pinyin::split_initial_final(ascii) {
        ("", final_) => final_.into(),
        (initial, final_) => format!(
            "{}{}",
            map_initial(initial).unwrap_or_else(|| {
                debug_assert_eq!(initial.len(), 1);
                initial
            }),
            map_final(final_).unwrap()
        )
        .as_str()
        .into(),
    }
}

#[rustfmt::skip]
fn ascii_to_diletter_abc(ascii: &str) -> PinyinString {
    ascii_to_diletter(
        ascii,
        |pinyin| Some(match pinyin {
            "e" => "oe", "o" => "oo",
            "a" => "oa",
            "ei" => "oq",
            "ai" => "ol",
            "ou" => "ob",
            "ao" => "ok",
            "en" => "of",
            "an" => "oj",
            "eng" => "og",
            "ang" => "oh",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |initial| Some(match initial {
            "zh" => "a", "ch" => "e", "sh" => "v",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |final_| Some(match final_ {
            "i" => "i", "u" => "u", "v" => "v",
            "e" => "e", "ie" => "x", "o" => "o", "uo" => "o", "ue" => "m", "ve" => "m",
            "a" => "a", "ia" => "d", "ua" => "d",
            "ei" => "q", "ui" => "m",
            "ai" => "l", "uai" => "c",
            "ou" => "b", "iu" => "r",
            "ao" => "k", "iao" => "z",
            "in" => "c", "un" => "n", "vn" => "n",
            "en" => "f",
            "an" => "j", "ian" => "w", "uan" => "p", "van" => "p",
            "ing" => "y",
            "ong" => "s", "iong" => "s",
            "eng" => "g",
            "ang" => "h", "iang" => "t", "uang" => "t",
            "er" => "or",
            _ => "",
        }).filter(|s| !s.is_empty()),
    )
}

#[rustfmt::skip]
fn ascii_to_diletter_jiajia(ascii: &str) -> PinyinString {
    ascii_to_diletter(
        ascii,
        |pinyin| Some(match pinyin {
            "e" => "ee", "o" => "oo",
            "a" => "aa",
            "ei" => "ew",
            "ai" => "as",
            "ou" => "op",
            "ao" => "ad",
            "en" => "er",
            "an" => "af",
            "eng" => "et",
            "ang" => "ag",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |initial| Some(match initial {
            "zh" => "v", "ch" => "u", "sh" => "i",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |final_| Some(match final_ {
            "i" => "i", "u" => "u", "v" => "v",
            "e" => "e", "ie" => "m", "o" => "o", "uo" => "o", "ue" => "x", "ve" => "t",
            "a" => "a", "ia" => "b", "ua" => "b",
            "ei" => "w", "ui" => "v",
            "ai" => "s", "uai" => "x",
            "ou" => "p", "iu" => "n",
            "ao" => "d", "iao" => "k",
            "in" => "l", "un" => "z", "vn" => "z",
            "en" => "r",
            "an" => "f", "ian" => "j", "uan" => "c", "van" => "c",
            "ing" => "q",
            "ong" => "y", "iong" => "y",
            "eng" => "t",
            "ang" => "g", "iang" => "h", "uang" => "h",
            "er" => "eq",
            _ => "",
        }).filter(|s| !s.is_empty()),
    )
}

#[rustfmt::skip]
fn ascii_to_diletter_microsoft(ascii: &str) -> PinyinString {
    ascii_to_diletter(
        ascii,
        |pinyin| Some(match pinyin {
            "e" => "oe", "o" => "oo",
            "a" => "oa",
            "ei" => "oz",
            "ai" => "ol",
            "ou" => "ob",
            "ao" => "ok",
            "en" => "of",
            "an" => "oj",
            "eng" => "og",
            "ang" => "oh",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |initial| Some(match initial {
            "zh" => "v", "ch" => "i", "sh" => "u",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |final_| Some(match final_ {
            "i" => "i", "u" => "u", "v" => "y",
            "e" => "e", "ie" => "x", "o" => "o", "uo" => "o", "ue" => "t", "ve" => "v",
            "a" => "a", "ia" => "w", "ua" => "w",
            "ei" => "z", "ui" => "v",
            "ai" => "l", "uai" => "y",
            "ou" => "b", "iu" => "q",
            "ao" => "k", "iao" => "c",
            "in" => "n", "un" => "p", "vn" => "p",
            "en" => "f",
            "an" => "j", "ian" => "m", "uan" => "r", "van" => "r",
            "ing" => ";",
            "ong" => "s", "iong" => "s",
            "eng" => "g",
            "ang" => "h", "iang" => "d", "uang" => "d",
            "er" => "or",
            _ => "",
        }).filter(|s| !s.is_empty()),
    )
}

#[rustfmt::skip]
fn ascii_to_diletter_thunisoft(ascii: &str) -> PinyinString {
    ascii_to_diletter(
        ascii,
        |pinyin| Some(match pinyin {
            "e" => "oe", "o" => "oo",
            "a" => "oa",
            "ei" => "ok",
            "ai" => "op",
            "ou" => "oz",
            "ao" => "oq",
            "en" => "ow",
            "an" => "or",
            "eng" => "ot",
            "ang" => "os",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |initial| Some(match initial {
            "zh" => "u", "ch" => "a", "sh" => "i",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |final_| Some(match final_ {
            "i" => "i", "u" => "u", "v" => "v",
            "e" => "e", "ie" => "d", "o" => "o", "uo" => "o", "ue" => "n", "ve" => "n",
            "a" => "a", "ia" => "x", "ua" => "x",
            "ei" => "k", "ui" => "n",
            "ai" => "p", "uai" => "y",
            "ou" => "z", "iu" => "j",
            "ao" => "q", "iao" => "b",
            "in" => "y", "un" => "m", "vn" => "y",
            "en" => "w",
            "an" => "r", "ian" => "f", "uan" => "l", "van" => "l",
            "ing" => ";",
            "ong" => "h", "iong" => "h",
            "eng" => "t",
            "ang" => "s", "iang" => "g", "uang" => "g",
            "er" => "oj",
            _ => "",
        }).filter(|s| !s.is_empty()),
    )
}

#[rustfmt::skip]
fn ascii_to_diletter_xiaohe(ascii: &str) -> PinyinString {
    ascii_to_diletter(
        ascii,
        |pinyin| Some(match pinyin {
            "e" => "ee", "o" => "oo",
            "a" => "aa",
            "ei" => "ei",
            "ai" => "ai",
            "ou" => "ou",
            "ao" => "ao",
            "en" => "en",
            "an" => "an",
            "eng" => "eg",
            "ang" => "ah",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |initial| Some(match initial {
            "zh" => "v", "ch" => "i", "sh" => "u",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |final_| Some(match final_ {
            "i" => "i", "u" => "u", "v" => "v",
            "e" => "e", "ie" => "p", "o" => "o", "uo" => "o", "ue" => "t", "ve" => "t",
            "a" => "a", "ia" => "x", "ua" => "x",
            "ei" => "w", "ui" => "v",
            "ai" => "d", "uai" => "k",
            "ou" => "z", "iu" => "q",
            "ao" => "c", "iao" => "n",
            "in" => "b", "un" => "y", "vn" => "y",
            "en" => "f",
            "an" => "j", "ian" => "m", "uan" => "r", "van" => "r",
            "ing" => "k",
            "ong" => "s", "iong" => "s",
            "eng" => "g",
            "ang" => "h", "iang" => "l", "uang" => "l",
            "er" => "er",
            _ => "",
        }).filter(|s| !s.is_empty()),
    )
}

#[rustfmt::skip]
fn ascii_to_diletter_zrm(ascii: &str) -> PinyinString {
    ascii_to_diletter(
        ascii,
        |pinyin| Some(match pinyin {
            "e" => "ee", "o" => "oo",
            "a" => "aa",
            "ei" => "ei",
            "ai" => "ai",
            "ou" => "ou",
            "ao" => "ao",
            "en" => "en",
            "an" => "an",
            "eng" => "eg",
            "ang" => "ah",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |initial| Some(match initial {
            "zh" => "v", "ch" => "i", "sh" => "u",
            _ => "",
        }).filter(|s| !s.is_empty()),
        |final_| Some(match final_ {
            "i" => "i", "u" => "u", "v" => "v",
            "e" => "e", "ie" => "x", "o" => "o", "uo" => "o", "ue" => "t", "ve" => "t",
            "a" => "a", "ia" => "w", "ua" => "w",
            "ei" => "z", "ui" => "v",
            "ai" => "l", "uai" => "y",
            "ou" => "b", "iu" => "q",
            "ao" => "k", "iao" => "c",
            "in" => "n", "un" => "p", "vn" => "p",
            "en" => "f",
            "an" => "j", "ian" => "m", "uan" => "r", "van" => "r",
            "ing" => ";",
            "ong" => "s", "iong" => "s",
            "eng" => "g",
            "ang" => "h", "iang" => "d", "uang" => "d",
            "er" => "er",
            _ => "",
        }).filter(|s| !s.is_empty()),
    )
}

#[cfg(test)]
mod tests {
    use super::{super::data, *};

    #[test]
    fn lowercase() {
        for unicode in data::PINYINS {
            assert_eq!(unicode, unicode.to_lowercase());
        }
    }

    #[test]
    fn max_len() {
        for unicode in data::PINYINS {
            let ascii = unicode_to_ascii(unicode);
            println!("{}: {}", unicode, ascii);

            assert!(unicode.len() <= 7, "{}", unicode.len());
            assert!(ascii.len() <= 6);

            assert_eq!(ascii_to_diletter_abc(&ascii).len(), 2);
            assert_eq!(ascii_to_diletter_jiajia(&ascii).len(), 2);
            assert_eq!(ascii_to_diletter_microsoft(&ascii).len(), 2);
            assert_eq!(ascii_to_diletter_thunisoft(&ascii).len(), 2);
            assert_eq!(ascii_to_diletter_xiaohe(&ascii).len(), 2);
            assert_eq!(ascii_to_diletter_zrm(&ascii).len(), 2);
        }
    }

    #[test]
    fn unicode_to_ascii_() {
        for unicode in data::PINYINS {
            let ascii = unicode_to_ascii(unicode);
            println!("{}: {}", unicode, ascii);

            assert!(ascii.len() <= 6);
        }
    }

    #[test]
    fn ascii_to_diletter_microsoft_() {
        assert_eq!(&ascii_to_diletter_microsoft("pin"), "pn");
        assert_eq!(&ascii_to_diletter_microsoft("ying"), "y;");
    }

    #[test]
    fn ascii_to_diletter_xiaohe_() {
        assert_eq!(&ascii_to_diletter_xiaohe("pin"), "pb");
        assert_eq!(&ascii_to_diletter_xiaohe("yin"), "yb");
    }
}

use std::marker::PhantomData;

use bon::bon;

use crate::{
    matcher::{encoding::EncodedStr, matches::SubMatch},
    unicode::{CharToMonoLowercase, StrToMonoLowercase},
};

pub mod analyze;
pub mod encoding;
mod matches;
#[cfg(feature = "regex")]
mod regex_utils;

#[cfg(feature = "pinyin")]
mod pinyin;
#[cfg(feature = "romaji")]
mod romaji;

pub use matches::Match;
#[cfg(feature = "pinyin")]
pub use pinyin::*;
#[cfg(feature = "romaji")]
pub use romaji::*;

enum AsciiMatcher {
    /// - find_ascii_too_short: +170%
    ///   - TODO
    /// - is_match_ascii -50%
    /// - find_ascii -55%
    /// - build -60%, `build_analyze` -25%
    /// - Build size -837.5 KiB
    Ac(aho_corasick::AhoCorasick),
    #[cfg(feature = "regex")]
    #[allow(unused)]
    Regex(regex::bytes::Regex),
}

struct PatternChar<'a> {
    c: char,
    c_lowercase: char,
    s: &'a str,
    s_lowercase: &'a str,
}

/// ## Design
/// API follows [`regex::Regex`](https://docs.rs/regex/latest/regex/struct.Regex.html).
///
/// ## Performance
/// - If you need to build [`IbMatcher`] multiple times, pass [`PinyinMatchConfigBuilder::data`] to the builder to avoid re-initializing the pinyin data every time.
/// - For matching more than 1000 strings, enable [`IbMatcherBuilder::analyze`] to optimize the pattern further. (The analysis costs ~65us, equivalent to about 220~1100 matches.)
///
/// TODO: No-pinyin pattern optimization
/// TODO: Anchors, `*_at`
/// TODO: Unicode normalization
/// TODO: No-hanzi haystack optimization (0.2/0.9%)
/// TODO: If pattern doesn't contain `.`, only match before `.` in the haystack
pub struct IbMatcher<'a, HaystackStr = str>
where
    HaystackStr: EncodedStr + ?Sized,
{
    /// For ASCII-only haystack optimization.
    ascii: Option<AsciiMatcher>,

    pattern: Box<[PatternChar<'a>]>,
    _pattern_string: String,
    _pattern_string_lowercase: String,

    min_haystack_len: usize,

    case_insensitive: bool,

    #[cfg(feature = "pinyin")]
    pinyin: Option<PinyinMatcher<'a>>,
    #[cfg(feature = "romaji")]
    romaji: Option<RomajiMatcher<'a>>,

    _haystack_str: PhantomData<HaystackStr>,
}

#[bon]
impl<'a, HaystackStr> IbMatcher<'a, HaystackStr>
where
    HaystackStr: EncodedStr + ?Sized,
{
    #[builder]
    pub fn new(
        #[builder(start_fn)] pattern: &HaystackStr,

        /// For more advanced control over the analysis, use [`IbMatcherBuilder::analyze_config`].
        #[builder(default = false)]
        analyze: bool,
        analyze_config: Option<analyze::PatternAnalyzeConfig>,

        /// The case insensitivity of pinyin is controlled by [`PinyinMatchConfigBuilder::case_insensitive`].
        #[builder(default = true)]
        case_insensitive: bool,

        /// If `true`, the pattern can match pinyins/romajis starting with the ending of the pattern.
        ///
        /// For example, pattern "pinyi" can match "拼音" (whose pinyin is "pinyin") if `is_pattern_partial` is `true`.
        #[builder(default = false)]
        is_pattern_partial: bool,

        #[cfg(feature = "pinyin")] pinyin: Option<PinyinMatchConfig<'a>>,
        #[cfg(feature = "romaji")] romaji: Option<RomajiMatchConfig<'a>>,
    ) -> Self {
        let pattern_bytes = pattern.as_bytes().to_owned();
        let pattern: String = pattern.char_index_strs().map(|(_, c, _)| c).collect();

        let pattern_string = pattern;
        let pattern_s: &str = pattern_string.as_str();
        let pattern_s: &'static str = unsafe { std::mem::transmute(pattern_s) };

        let pattern_string_lowercase = pattern_string.to_mono_lowercase();
        let pattern_s_lowercase: &str = pattern_string_lowercase.as_str();
        let pattern_s_lowercase: &'static str = unsafe { std::mem::transmute(pattern_s_lowercase) };

        let pattern = pattern_string
            .char_indices()
            .zip(pattern_string_lowercase.char_indices())
            .map(|((i, c), (i_lowercase, c_lowercase))| {
                debug_assert_eq!(i, i_lowercase);
                PatternChar {
                    c,
                    c_lowercase,
                    s: &pattern_s[i..],
                    s_lowercase: &pattern_s_lowercase[i..],
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        #[cfg(feature = "pinyin")]
        if let Some(pinyin) = &pinyin {
            // TODO: If pattern does not contain any pinyin letter, then pinyin_data is not needed.
            #[cfg(not(feature = "inmut-data"))]
            assert!(pinyin.data.inited_notations().contains(pinyin.notations));
            #[cfg(feature = "inmut-data")]
            pinyin.data.init_notations(pinyin.notations);
        }

        let analyzer = analyze::PatternAnalyzer::builder(pattern_s_lowercase);
        #[cfg(feature = "pinyin")]
        let analyzer = analyzer.maybe_pinyin(pinyin.as_ref());
        let mut analyzer = analyzer.build();
        analyzer.analyze(analyze_config.unwrap_or_else(|| {
            if analyze {
                analyze::PatternAnalyzeConfig::standard()
            } else {
                analyze::PatternAnalyzeConfig::default()
            }
        }));

        let min_haystack_len = match HaystackStr::ELEMENT_LEN_BYTE {
            1 => analyzer.min_haystack_len(),
            _ if pattern.is_empty() => 0,
            len => {
                // TODO
                len
            }
        };

        #[cfg(feature = "pinyin")]
        let pinyin_analyze = analyzer.pinyin().clone();
        // TODO: Optimize if only AsciiFirstLetter is used

        drop(analyzer);

        #[cfg(feature = "pinyin")]
        let pinyin = pinyin.map(|config| {
            PinyinMatcher::builder(config)
                .analyze(pinyin_analyze)
                .is_pattern_partial(is_pattern_partial)
                .build()
        });

        // ASCII-only haystack optimization
        let ascii = match pattern_bytes.is_ascii() {
            true => Some(
                // regex::bytes::RegexBuilder::new(&regex_utils::escape_bytes(&pattern_bytes))
                //     .unicode(false)
                //     .case_insensitive(case_insensitive)
                //     .build()
                //     .unwrap(),
                AsciiMatcher::Ac(
                    aho_corasick::AhoCorasick::builder()
                        .ascii_case_insensitive(case_insensitive)
                        .build(&[pattern_bytes])
                        .unwrap(),
                ),
            ),
            // ASCII-only haystack with non-ASCII pattern optimization
            false => None,
        };

        Self {
            ascii,

            min_haystack_len,

            pattern,
            _pattern_string: pattern_string,
            _pattern_string_lowercase: pattern_string_lowercase,

            case_insensitive,

            #[cfg(feature = "pinyin")]
            pinyin,

            #[cfg(feature = "romaji")]
            romaji: romaji.map(|config| RomajiMatcher {
                partial_pattern: is_pattern_partial && config.allow_partial_pattern,
                config,
            }),

            _haystack_str: PhantomData,
        }
    }

    /// This routine searches for the first match of this pattern in the haystack given, and if found, returns a [`Match`]. The [`Match`] provides access to both the byte offsets of the match and [`Match::is_pattern_partial()`].
    ///
    /// Note that this should only be used if you want to find the entire match. If instead you just want to test the existence of a match, it’s potentially faster to use [`IbMatcher::is_match()`] instead of `IbMatcher::find().is_some()`.
    pub fn find(&self, haystack: &HaystackStr) -> Option<Match> {
        self.find_with_is_ascii(haystack, haystack.is_ascii())
    }

    fn find_with_is_ascii(&self, haystack: &HaystackStr, is_ascii: bool) -> Option<Match> {
        if self.pattern.is_empty() {
            return Some(Match {
                start: 0,
                end: 0,
                is_pattern_partial: false,
            });
        }

        if is_ascii {
            return self
                .ascii
                .as_ref()
                .map(|ascii| match ascii {
                    AsciiMatcher::Ac(ac) => ac.find(haystack.as_bytes()).map(|m| Match {
                        start: m.start() / HaystackStr::ELEMENT_LEN_BYTE,
                        end: m.end() / HaystackStr::ELEMENT_LEN_BYTE,
                        is_pattern_partial: false,
                    }),
                    #[cfg(feature = "regex")]
                    AsciiMatcher::Regex(regex) => regex.find(haystack.as_bytes()).map(|m| Match {
                        start: m.start() / HaystackStr::ELEMENT_LEN_BYTE,
                        end: m.end() / HaystackStr::ELEMENT_LEN_BYTE,
                        is_pattern_partial: false,
                    }),
                })
                .flatten();
        }

        for (i, _c, str) in haystack.char_index_strs() {
            if self.is_haystack_too_short(str) {
                break;
            }
            if let Some(submatch) = self.sub_test(&self.pattern, str, 0) {
                return Some(Match {
                    start: i,
                    end: i + submatch.len,
                    is_pattern_partial: submatch.is_pattern_partial,
                });
            }
        }

        None
    }

    /// Returns true if and only if there is a match for the pattern anywhere in the haystack given.
    ///
    /// It is recommended to use this method if all you need to do is test whether a match exists, since the underlying matching engine may be able to do less work.
    pub fn is_match(&self, haystack: &HaystackStr) -> bool {
        if haystack.is_ascii() {
            return self
                .ascii
                .as_ref()
                .map(|ascii| match ascii {
                    AsciiMatcher::Ac(ac) => ac.is_match(haystack.as_bytes()),
                    #[cfg(feature = "regex")]
                    AsciiMatcher::Regex(regex) => regex.is_match(haystack.as_bytes()),
                })
                .unwrap_or(false);
        }

        self.find_with_is_ascii(haystack, false).is_some()
    }

    /// This routine tests if this pattern matches the haystack at the start, and if found, returns a [`Match`]. The [`Match`] provides access to both the byte offsets of the match and [`Match::is_pattern_partial()`].
    ///
    /// ## Returns
    /// - `Match.start()` is guaranteed to be 0.
    /// - If there are multiple possible matches, the longer ones are preferred. But the result is not guaranteed to be the longest one.
    pub fn test(&self, haystack: &HaystackStr) -> Option<Match> {
        if self.is_haystack_too_short(haystack) {
            return None;
        } else {
            if self.pattern.is_empty() {
                return Some(Match {
                    start: 0,
                    end: 0,
                    is_pattern_partial: false,
                });
            }
        }

        if haystack.is_ascii() {
            return self
                .ascii
                .as_ref()
                .map(|ascii| match ascii {
                    AsciiMatcher::Ac(ac) => ac
                        .find(haystack.as_bytes())
                        .filter(|m| m.start() == 0)
                        .map(|m| Match {
                            start: 0,
                            end: m.end() / HaystackStr::ELEMENT_LEN_BYTE,
                            is_pattern_partial: false,
                        }),
                    // TODO: Use regex-automata's anchored searches?
                    #[cfg(feature = "regex")]
                    AsciiMatcher::Regex(regex) => regex
                        .find(haystack.as_bytes())
                        .filter(|m| m.start() == 0)
                        .map(|m| Match {
                            start: 0,
                            end: m.end() / HaystackStr::ELEMENT_LEN_BYTE,
                            is_pattern_partial: false,
                        }),
                })
                .flatten();
        }

        self.sub_test(&self.pattern, haystack, 0)
            .map(|submatch| Match {
                start: 0,
                end: submatch.len,
                is_pattern_partial: submatch.is_pattern_partial,
            })
    }

    /// ## Arguments
    /// - `pattern`: Not empty.
    /// - `haystack`
    /// - `matched_len`: For tail-call optimization.
    fn sub_test(
        &self,
        pattern: &[PatternChar],
        haystack: &HaystackStr,
        matched_len: usize,
    ) -> Option<SubMatch> {
        debug_assert!(!pattern.is_empty());

        // if Self::is_haystack_too_short_with_pattern(pattern, haystack) {
        //     return None;
        // }

        let (haystack_c, haystack_c_len, haystack_next) = {
            match haystack.char_len_next_strs().next() {
                Some(v) => v,
                None => {
                    return None;

                    // // pattern is not empty, so haystack must not be empty too.
                    // unsafe { unreachable_unchecked() }
                }
            }
        };
        let matched_len_next = matched_len + haystack_c_len;

        let (pattern_c, pattern_next) = pattern.split_first().unwrap();

        if match self.case_insensitive {
            true => haystack_c.to_mono_lowercase() == pattern_c.c_lowercase,
            false => haystack_c == pattern_c.c,
        } {
            // If haystack_c == pattern_c, then it is impossible that pattern_c is a pinyin letter and haystack_c is a hanzi.
            return if pattern_next.is_empty() {
                Some(SubMatch::new(matched_len_next, false))
            } else {
                self.sub_test(pattern_next, haystack_next, matched_len_next)
            };
        }

        // Fast fail optimization
        #[cfg(any(feature = "pinyin", feature = "romaji"))]
        if haystack_c.is_ascii() {
            return None;
        }

        #[cfg(feature = "romaji")]
        if let Some(romaji) = &self.romaji {
            // const {
            //     assert!(
            //         HaystackStr::ELEMENT_LEN_BYTE == 1,
            //         "non-UTF-8 romaji match is not yet supported"
            //     );
            // }
            debug_assert_eq!(
                HaystackStr::ELEMENT_LEN_BYTE,
                1,
                "non-UTF-8 romaji match is not yet supported"
            );
            if let Some(m) = romaji.config.romanizer.romanize_and_try_for_each(
                unsafe { str::from_utf8_unchecked(haystack.as_bytes()) },
                |len, romaji| {
                    let match_len_next = matched_len + len;
                    match self.sub_test_pinyin::<1>(
                        pattern,
                        unsafe { haystack.get_unchecked_from(len..) },
                        match_len_next,
                        romaji,
                    ) {
                        (true, Some(submatch)) => return Some(submatch),
                        (true, None) => (),
                        (false, None) => (),
                        (false, Some(_)) => unreachable!(),
                    }
                    None
                },
            ) {
                return Some(m);
            }
        }

        #[cfg(feature = "pinyin")]
        if let Some(matcher) = &self.pinyin {
            // for pinyin in self.pinyin_data.get_pinyins(haystack_c) {
            //     for &notation in self.pinyin.notations_prefix_group.iter() {
            //         let pinyin = pinyin.notation(notation).unwrap();
            //         match self.sub_test_pinyin(pattern, haystack_next, matched_len, pinyin) {
            //             (true, Some(submatch)) => return Some(submatch),
            //             (true, None) => (),
            //             (false, None) => break,
            //             (false, Some(_)) => unreachable!(),
            //         }
            //     }
            //     for &notation in self.pinyin.notations.iter() {
            //         let pinyin = pinyin.notation(notation).unwrap();
            //         match self.sub_test_pinyin(pattern, haystack_next, matched_len, pinyin) {
            //             (true, Some(submatch)) => return Some(submatch),
            //             (true, None) => (),
            //             (false, None) => (),
            //             (false, Some(_)) => unreachable!(),
            //         }
            //     }
            // }
            // None

            // Reduce total time by 45~65% compared to using `get_pinyins()`
            if let Some(m) =
                matcher
                    .config
                    .data
                    .get_pinyins_and_try_for_each(haystack_c, |pinyin| {
                        for &notation in matcher.notations_prefix_group.iter() {
                            let pinyin = pinyin.notation(notation).unwrap();
                            match self.sub_test_pinyin::<0>(
                                pattern,
                                haystack_next,
                                matched_len_next,
                                pinyin,
                            ) {
                                (true, Some(submatch)) => return Some(submatch),
                                (true, None) => (),
                                (false, None) => break,
                                (false, Some(_)) => unreachable!(),
                            }
                        }
                        for &notation in matcher.notations.iter() {
                            let pinyin = pinyin.notation(notation).unwrap();
                            match self.sub_test_pinyin::<0>(
                                pattern,
                                haystack_next,
                                matched_len_next,
                                pinyin,
                            ) {
                                (true, Some(submatch)) => return Some(submatch),
                                (true, None) => (),
                                (false, None) => (),
                                (false, Some(_)) => unreachable!(),
                            }
                        }
                        None
                    })
            {
                return Some(m);
            }
        }

        None
    }

    /// ## Arguments
    /// - `pattern`: Not empty.
    /// - `haystack`
    /// - `matched_len`: For tail-call optimization.
    ///
    /// ## Returns
    /// (pinyin_matched, submatch)
    fn sub_test_pinyin<const LANG: u8>(
        &self,
        pattern: &[PatternChar],
        haystack_next: &HaystackStr,
        matched_len_next: usize,
        pinyin: &str,
    ) -> (bool, Option<SubMatch>) {
        debug_assert!(!pattern.is_empty());
        debug_assert_eq!(pinyin, pinyin.to_lowercase());

        let pattern_s = match match LANG {
            #[cfg(feature = "pinyin")]
            0 => {
                unsafe { self.pinyin.as_ref().unwrap_unchecked() }
                    .config
                    .case_insensitive
            }
            #[cfg(feature = "romaji")]
            1 => {
                unsafe { self.romaji.as_ref().unwrap_unchecked() }
                    .config
                    .case_insensitive
            }
            _ => unreachable!(),
        } {
            true => pattern[0].s_lowercase,
            false => pattern[0].s,
        };

        if pattern_s.len() < pinyin.len() {
            if match LANG {
                #[cfg(feature = "pinyin")]
                0 => unsafe { self.pinyin.as_ref().unwrap_unchecked() }.partial_pattern,
                #[cfg(feature = "romaji")]
                1 => unsafe { self.romaji.as_ref().unwrap_unchecked() }.partial_pattern,
                _ => unreachable!(),
            } && pinyin.starts_with(pattern_s)
            {
                return (true, Some(SubMatch::new(matched_len_next, true)));
            }
        } else if pattern_s.starts_with(pinyin) {
            if pattern_s.len() == pinyin.len() {
                return (true, Some(SubMatch::new(matched_len_next, false)));
            }

            if let Some(submatch) = self.sub_test(
                &pattern[pinyin.chars().count()..],
                haystack_next,
                matched_len_next,
            ) {
                return (true, Some(submatch));
            }

            return (true, None);
        }

        (false, None)
    }

    // /// Reduce ~10% miss case time at the cost of some hit case time.
    // fn is_haystack_too_short_with_pattern(
    //     _pattern: &[PatternChar],
    //     _haystack: &HaystackStr,
    // ) -> bool {
    //     // For hit case:
    //     // - ~~A PatternChar must at least match one char in the haystack, i.e. `haystack.chars_count() >= pattern.len()`~~
    //     //  - ~~So `haystack.len() >= haystack.chars_count() >= pattern.len()`~~
    //     // - pattern.len() and pattern.s.len() may be shorter, equal, or longer than haystack.len()
    //     //   - We have pinyin that is longer than its hanzi, like "shuang".len() > "双".len()

    //     // haystack.chars_count() < pattern.len()
    //     // haystack.as_bytes().len() < pattern.len()
    //     false
    // }

    /// Already tested in match methods.
    pub fn is_haystack_too_short(&self, haystack: &HaystackStr) -> bool {
        // Self::is_haystack_too_short_with_pattern(&self.pattern, haystack)
        haystack.as_bytes().len() < self.min_haystack_len
    }
}

#[cfg(test)]
mod test {
    use crate::pinyin::PinyinNotation;

    use super::*;

    #[macro_export]
    macro_rules! assert_match {
        ($m:expr, $expected:expr) => {
            assert_eq!($m.map(|m| (m.start(), m.len())), $expected);
        };
    }

    fn assert_match(m: Option<Match>, expected: Option<(usize, usize)>) {
        assert_eq!(m.map(|m| (m.start(), m.len())), expected);
    }

    #[test]
    fn is_haystack_too_short() {
        // assert!(IbMatcher::is_haystack_too_short_with_pattern(&[], "") == false);
        // assert!(IbMatcher::is_haystack_too_short_with_pattern(&[], "a") == false);

        let matcher = IbMatcher::builder("pysseve")
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
            .analyze(true)
            .build();
        assert!(matcher.is_haystack_too_short(""));
        assert!(matcher.is_haystack_too_short("a"));
        assert!(matcher.is_haystack_too_short("pyss"));
        assert!(matcher.is_haystack_too_short("pyssEverything") == false);
        assert!(matcher.is_haystack_too_short("拼"));
        assert!(matcher.is_haystack_too_short("拼音"));
        assert!(matcher.is_haystack_too_short("拼音搜") == false);
    }

    #[test]
    fn test() {
        let matcher = IbMatcher::builder("xing")
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
            .build();
        assert_match(matcher.test(""), None);
        assert_match(matcher.test("xing"), Some((0, 4)));
        assert_match(matcher.test("XiNG"), Some((0, 4)));
        assert_match(matcher.test("行"), Some((0, 3)));

        let matcher = IbMatcher::builder("ke")
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
            .build();
        assert_match(matcher.test("ke"), Some((0, 2)));
        assert_match(matcher.test("科"), Some((0, 3)));
        assert_match(matcher.test("k鹅"), Some((0, 4)));
        assert_match(matcher.test("凯尔"), None);

        let matcher = IbMatcher::builder("")
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
            .build();
        assert_match(matcher.test(""), Some((0, 0)));
        assert_match(matcher.test("abc"), Some((0, 0)));

        let matcher = IbMatcher::builder("ke")
            .pinyin(PinyinMatchConfig::notations(
                PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
            ))
            .build();
        assert_match(matcher.test("ke"), Some((0, 2)));
        assert_match(matcher.test("科"), Some((0, 3)));
        assert_match(matcher.test("k鹅"), Some((0, 4)));
        assert_match(matcher.test("凯尔"), Some((0, 6)));
        // AsciiFirstLetter is preferred
        assert_match(matcher.test("柯尔"), Some((0, 6)));
    }

    #[cfg(feature = "encoding")]
    #[test]
    fn test_u16() {
        use widestring::u16str;

        let matcher = IbMatcher::builder(u16str!("xing"))
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
            .build();
        // assert_match(matcher.test(u16str!("")), None);

        assert_match(matcher.test(u16str!("xing")), Some((0, 4)));

        assert_match(matcher.test(u16str!("XiNG")), Some((0, 4)));
        assert_match(matcher.test(u16str!("行")), Some((0, 1)));

        let matcher = IbMatcher::builder(u16str!("ke"))
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
            .build();
        assert_match(matcher.test(u16str!("ke")), Some((0, 2)));
        assert_match(matcher.test(u16str!("科")), Some((0, 1)));
        assert_match(matcher.test(u16str!("k鹅")), Some((0, 2)));
        assert_match(matcher.test(u16str!("凯尔")), None);

        let matcher = IbMatcher::builder(u16str!(""))
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
            .build();
        assert_match(matcher.test(u16str!("")), Some((0, 0)));
        assert_match(matcher.test(u16str!("abc")), Some((0, 0)));

        let matcher = IbMatcher::builder(u16str!("ke"))
            .pinyin(PinyinMatchConfig::notations(
                PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
            ))
            .build();
        assert_match(matcher.test(u16str!("ke")), Some((0, 2)));
        assert_match(matcher.test(u16str!("科")), Some((0, 1)));
        assert_match(matcher.test(u16str!("k鹅")), Some((0, 2)));
        assert_match(matcher.test(u16str!("凯尔")), Some((0, 2)));
        // AsciiFirstLetter is preferred
        assert_match(matcher.test(u16str!("柯尔")), Some((0, 2)));
    }

    #[test]
    fn test_case_insensitive() {
        let matcher = IbMatcher::builder("xing")
            .case_insensitive(false)
            .pinyin(
                PinyinMatchConfig::builder(PinyinNotation::Ascii)
                    .case_insensitive(false)
                    .build(),
            )
            .build();
        assert_match(matcher.test("xing"), Some((0, 4)));
        assert_match(matcher.test("XiNG"), None);
        assert_match(matcher.test("行"), Some((0, 3)));

        let matcher = IbMatcher::builder("XING")
            .case_insensitive(true)
            .pinyin(
                PinyinMatchConfig::builder(PinyinNotation::Ascii)
                    .case_insensitive(false)
                    .build(),
            )
            .build();
        assert_match(matcher.test("xing"), Some((0, 4)));
        assert_match(matcher.test("XiNG"), Some((0, 4)));
        assert_match(matcher.test("行"), None);

        let matcher = IbMatcher::builder("XING")
            .case_insensitive(true)
            .pinyin(
                PinyinMatchConfig::builder(PinyinNotation::Ascii)
                    .case_insensitive(true)
                    .build(),
            )
            .build();
        assert_match(matcher.test("xing"), Some((0, 4)));
        assert_match(matcher.test("XiNG"), Some((0, 4)));
        assert_match(matcher.test("行"), Some((0, 3)));

        let matcher = IbMatcher::builder("XiNG")
            .case_insensitive(false)
            .pinyin(
                PinyinMatchConfig::builder(PinyinNotation::Ascii)
                    .case_insensitive(true)
                    .build(),
            )
            .build();
        assert_match(matcher.test("xing"), None);
        assert_match(matcher.test("XiNG"), Some((0, 4)));
        assert_match(matcher.test("行"), Some((0, 3)));
    }

    #[test]
    fn find() {
        let matcher = IbMatcher::builder("xing")
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
            .build();
        assert_match(matcher.find(""), None);
        assert_match(matcher.find("buxing"), Some((2, 4)));
        assert_match(matcher.find("BuXiNG"), Some((2, 4)));
        assert_match(matcher.find("不行"), Some((3, 3)));

        let matcher = IbMatcher::builder("")
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
            .build();
        assert_match(matcher.find(""), Some((0, 0)));
        assert_match(matcher.find("abc"), Some((0, 0)));
    }
}

use std::iter;

use aho_corasick::{automaton::Automaton, Anchored, StartKind};
use bon::{bon, Builder};

use crate::matcher::Match;

/// Note [`PlainMatchConfigBuilder::case_insensitive`] is `true` by default, unlike [`PinyinMatchConfigBuilder`] and [`RomajiMatchConfigBuilder`].
#[derive(Builder, Clone, Debug)]
pub struct PlainMatchConfig {
    /// The case insensitivity of pinyin is controlled by [`PinyinMatchConfigBuilder::case_insensitive`].
    #[builder(default = true)]
    pub(crate) case_insensitive: bool,

    #[builder(default = true, setters(vis = "pub(crate)"))]
    pub(crate) maybe_ascii: bool,
}

impl PlainMatchConfig {
    pub(crate) fn case_insensitive(case_insensitive: bool) -> Option<Self> {
        Some(Self {
            case_insensitive,
            maybe_ascii: true,
        })
    }
}

/// For ASCII-only haystack optimization.
pub enum AsciiMatcher<const CHAR_LEN: usize = 1> {
    /// ASCII-only haystack with non-ASCII pattern optimization
    Fail,
    AcDFA(AcDfaMatcher),
    /// - find_ascii_too_short: +170%
    ///   - TODO
    /// - is_match_ascii -50%
    /// - find_ascii -55%
    /// - build -60%, `build_analyze` -25%
    /// - Build size -837.5 KiB
    #[cfg(feature = "perf-plain-ac")]
    Ac(AcMatcher),
    #[cfg(feature = "perf-plain-regex")]
    #[allow(unused)]
    Regex(regex::bytes::Regex),
}

use AsciiMatcher::*;

/// Almost the same as [`AcMatcher`], but without the `dyn` cost.
pub(crate) struct AcDfaMatcher {
    dfa: aho_corasick::dfa::DFA,
    /// `dfa` also has `start_state`, but here has free space so anyway
    starts_with: bool,
    ends_with: bool,

    /// For [`AsciiMatcher::test_single()`].
    pattern: Vec<u8>,
    case_insensitive: bool,
}

impl AcDfaMatcher {
    #[inline]
    pub fn input<'h>(&self, haystack: &'h [u8]) -> aho_corasick::Input<'h> {
        aho_corasick::Input::new(haystack).anchored(if self.starts_with {
            Anchored::Yes
        } else {
            Anchored::No
        })
    }
}

#[cfg(feature = "perf-plain-ac")]
pub(crate) struct AcMatcher {
    ac: aho_corasick::AhoCorasick,
    /// `ac` also has `start_kind`, but here has free space so anyway
    starts_with: bool,
    ends_with: bool,
}

#[cfg(feature = "perf-plain-ac")]
impl AcMatcher {
    #[inline]
    pub fn input<'h>(&self, haystack: &'h [u8]) -> aho_corasick::Input<'h> {
        aho_corasick::Input::new(haystack).anchored(if self.starts_with {
            Anchored::Yes
        } else {
            Anchored::No
        })
    }
}

#[bon]
impl<const CHAR_LEN: usize> AsciiMatcher<CHAR_LEN> {
    #[builder]
    pub fn new(
        #[builder(start_fn)] pattern: &[u8],
        plain: Option<&PlainMatchConfig>,
        #[builder(default = false)] starts_with: bool,
        #[builder(default = false)] ends_with: bool,
    ) -> Self {
        match plain.filter(|_| pattern.is_ascii()) {
            Some(plain) => {
                // regex::bytes::RegexBuilder::new(&regex_utils::escape_bytes(pattern))
                //     .unicode(false)
                //     .case_insensitive(case_insensitive)
                //     .build()
                //     .unwrap(),

                // Ac(AcMatcher {
                //     ac: aho_corasick::AhoCorasick::builder()
                //         .ascii_case_insensitive(plain.case_insensitive)
                //         .start_kind(if starts_with {
                //             StartKind::Anchored
                //         } else {
                //             StartKind::Unanchored
                //         })
                //         .build(&[pattern])
                //         .unwrap(),
                //     starts_with,
                //     ends_with,
                // })
                AcDFA(AcDfaMatcher {
                    dfa: aho_corasick::dfa::DFA::builder()
                        .ascii_case_insensitive(plain.case_insensitive)
                        .start_kind(if starts_with {
                            StartKind::Anchored
                        } else {
                            StartKind::Unanchored
                        })
                        .build(&[pattern])
                        .unwrap(),
                    starts_with,
                    ends_with,
                    pattern: if plain.case_insensitive {
                        pattern.to_ascii_lowercase()
                    } else {
                        pattern.into()
                    },
                    case_insensitive: plain.case_insensitive,
                })
            }
            None => Fail,
        }
    }

    pub fn find(&self, haystack: &[u8]) -> Option<Match> {
        match self {
            Fail => None,
            AcDFA(ac) => {
                if ac.ends_with {
                    let start = if ac.starts_with {
                        0
                    } else {
                        haystack.len().saturating_sub(ac.dfa.max_pattern_len())
                    };
                    ac.dfa
                        .try_find_iter(ac.input(&haystack[start..]))
                        // Only if Anchored doesn't match
                        .unwrap()
                        .filter(|m| start + m.end() == haystack.len())
                        .map(|m| Match {
                            start: start + m.start() / CHAR_LEN,
                            end: start + m.end() / CHAR_LEN,
                            is_pattern_partial: false,
                        })
                        .next()
                } else {
                    ac.dfa
                        .try_find(&ac.input(haystack))
                        .unwrap()
                        .map(|m| Match {
                            start: m.start() / CHAR_LEN,
                            end: m.end() / CHAR_LEN,
                            is_pattern_partial: false,
                        })
                }
            }
            #[cfg(feature = "perf-plain-ac")]
            Ac(ac) => {
                if ac.ends_with {
                    let start = if ac.starts_with {
                        0
                    } else {
                        haystack.len().saturating_sub(ac.ac.max_pattern_len())
                    };
                    ac.ac
                        .find_iter(ac.input(&haystack[start..]))
                        .filter(|m| start + m.end() == haystack.len())
                        .map(|m| Match {
                            start: start + m.start() / CHAR_LEN,
                            end: start + m.end() / CHAR_LEN,
                            is_pattern_partial: false,
                        })
                        .next()
                } else {
                    ac.ac.find(ac.input(haystack)).map(|m| Match {
                        start: m.start() / CHAR_LEN,
                        end: m.end() / CHAR_LEN,
                        is_pattern_partial: false,
                    })
                }
            }
            #[cfg(feature = "perf-plain-regex")]
            Regex(regex) => regex.find(haystack).map(|m| Match {
                start: m.start() / CHAR_LEN,
                end: m.end() / CHAR_LEN,
                is_pattern_partial: false,
            }),
        }
    }

    pub fn is_match(&self, haystack: &[u8]) -> bool {
        match self {
            Fail => false,
            AcDFA(ac) => {
                if ac.ends_with {
                    self.find(haystack).is_some()
                } else {
                    ac.dfa
                        .try_find(&ac.input(haystack).earliest(true))
                        .unwrap()
                        .is_some()
                }
            }
            #[cfg(feature = "perf-plain-ac")]
            Ac(ac) => {
                if ac.ends_with {
                    self.find(haystack).is_some()
                } else {
                    ac.ac.is_match(ac.input(haystack))
                }
            }
            #[cfg(feature = "perf-plain-regex")]
            Regex(regex) => regex.is_match(haystack),
        }
    }

    /// ~15% faster than [`AsciiMatcher::AcDFA`]
    #[inline(always)]
    fn test_single(
        &self,
        pattern: &[u8],
        ends_with: bool,
        case_insensitive: bool,
        haystack: &[u8],
    ) -> Option<Match> {
        let hay = haystack.get(..pattern.len())?;
        if ends_with && pattern.len() != haystack.len() {
            return None;
        }
        if if case_insensitive {
            // haystack.eq_ignore_ascii_case(&pattern)
            // TODO: Case map?
            iter::zip(pattern, hay).all(|(&a, b)| a == b.to_ascii_lowercase())
        } else {
            hay == pattern
        } {
            return Some(Match {
                start: 0,
                end: pattern.len() / CHAR_LEN,
                is_pattern_partial: false,
            });
        }
        None
    }

    pub fn test(&self, haystack: &[u8]) -> Option<Match> {
        match self {
            Fail => None,
            AcDFA(ac) => {
                // // TODO: Always use anchored?
                // let hay = haystack.get(..ac.dfa.max_pattern_len())?;
                // let input = ac.input(hay);
                // if ac.ends_with {
                //     ac.dfa
                //         .try_find(&input)
                //         .unwrap()
                //         .filter(|m| m.start() == 0 && m.end() == haystack.len())
                //         .map(|m| Match {
                //             start: 0,
                //             end: m.end() / CHAR_LEN,
                //             is_pattern_partial: false,
                //         })
                // } else {
                //     ac.dfa
                //         .try_find(&input)
                //         .unwrap()
                //         .filter(|m| m.start() == 0)
                //         .map(|m| Match {
                //             start: 0,
                //             end: m.end() / CHAR_LEN,
                //             is_pattern_partial: false,
                //         })
                // }
                self.test_single(&ac.pattern, ac.ends_with, ac.case_insensitive, haystack)
            }
            #[cfg(feature = "perf-plain-ac")]
            Ac(ac) => {
                // TODO: Always use anchored?
                let hay = haystack.get(..ac.ac.max_pattern_len())?;
                let input = ac.input(hay);
                if ac.ends_with {
                    ac.ac
                        .find(input)
                        .filter(|m| m.start() == 0 && m.end() == haystack.len())
                        .map(|m| Match {
                            start: 0,
                            end: m.end() / CHAR_LEN,
                            is_pattern_partial: false,
                        })
                } else {
                    ac.ac.find(input).filter(|m| m.start() == 0).map(|m| Match {
                        start: 0,
                        end: m.end() / CHAR_LEN,
                        is_pattern_partial: false,
                    })
                }
            }
            // TODO: Use regex-automata's anchored searches?
            #[cfg(feature = "perf-plain-regex")]
            Regex(regex) => regex
                .find(haystack)
                .filter(|m| m.start() == 0)
                .map(|m| Match {
                    start: 0,
                    end: m.end() / CHAR_LEN,
                    is_pattern_partial: false,
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_match;

    use super::*;

    #[test]
    fn ends_with() {
        let matcher = AsciiMatcher::<1>::builder(b"abc")
            .maybe_plain(PlainMatchConfig::case_insensitive(true).as_ref())
            .ends_with(true)
            .build();
        assert!(matcher.is_match(b"abc"));
        assert!(!matcher.is_match(b"ab"));
        assert_match!(matcher.find(b"abcd"), None);
        assert!(!matcher.is_match(b"abcd"));
        assert!(matcher.is_match(b"ABC"));
        assert_match!(matcher.find(b"xyzabc"), Some((3, 3)));
        assert!(matcher.is_match(b"xyzabc"));
        assert!(!matcher.is_match(b"xyzab"));

        let matcher = AsciiMatcher::<1>::builder(b"abc")
            .maybe_plain(PlainMatchConfig::case_insensitive(true).as_ref())
            .ends_with(false)
            .build();
        assert!(matcher.is_match(b"abc"));
        assert!(!matcher.is_match(b"ab"));
        assert!(matcher.is_match(b"abcd"));
        assert!(matcher.is_match(b"ABC"));
        assert!(matcher.is_match(b"xyzabc"));
        assert!(!matcher.is_match(b"xyzab"));
    }

    #[test]
    fn starts_with() {
        let matcher = AsciiMatcher::<1>::builder(b"abc")
            .maybe_plain(PlainMatchConfig::case_insensitive(true).as_ref())
            .starts_with(true)
            .build();
        assert!(matcher.is_match(b"abc"));
        assert!(!matcher.is_match(b"ab"));
        assert!(matcher.is_match(b"abcd"));
        assert!(matcher.is_match(b"ABC"));
        assert!(!matcher.is_match(b"xyzabc"));
        assert!(!matcher.is_match(b"xyzab"));

        let matcher = AsciiMatcher::<1>::builder(b"abc")
            .maybe_plain(PlainMatchConfig::case_insensitive(true).as_ref())
            .starts_with(false)
            .build();
        assert!(matcher.is_match(b"abc"));
        assert!(!matcher.is_match(b"ab"));
        assert!(matcher.is_match(b"abcd"));
        assert!(matcher.is_match(b"ABC"));
        assert!(matcher.is_match(b"xyzabc"));
        assert!(!matcher.is_match(b"xyzab"));
    }
}

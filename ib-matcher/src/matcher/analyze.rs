use std::cmp::min;

use bon::{bon, Builder};

#[cfg(feature = "romaji")]
use crate::matcher::RomajiMatchConfig;
#[cfg(feature = "pinyin")]
use crate::{
    matcher::{PinyinAnalyzeResult, PinyinMatchConfig},
    pinyin::PinyinNotation,
};

#[derive(Builder, Clone)]
pub struct PatternAnalyzeConfig {
    /// For better [`PatternAnalyzer::used_notations`] and [`PatternAnalyzer::min_haystack_len`].
    ///
    /// ~65us (+90%, 220~1100 matches)
    traversal: bool,
}

impl PatternAnalyzeConfig {
    pub fn standard() -> Self {
        Self { traversal: true }
    }
}

impl Default for PatternAnalyzeConfig {
    fn default() -> Self {
        Self { traversal: false }
    }
}

pub(crate) struct PatternAnalyzer<'a> {
    pattern: &'a str,
    is_pattern_partial: bool,

    #[cfg(feature = "pinyin")]
    pinyin: Option<&'a PinyinMatchConfig<'a>>,
    #[cfg(feature = "pinyin")]
    pinyin_result: PinyinAnalyzeResult,

    #[cfg(feature = "romaji")]
    romaji: Option<&'a RomajiMatchConfig<'a>>,

    traversal_count: usize,
    #[cfg(test)]
    min_haystack_chars: usize,
    /// TODO: Per lang len when `mix_lang` is false
    /// TODO: min_non_ascii_haystack_len
    min_haystack_len: usize,
}

#[bon]
impl<'a> PatternAnalyzer<'a> {
    /// - 100: 280 us (~1400 find 5, faster if >9333 haystacks)
    /// - 1000: 2.7 ms
    /// - 10000: 26.7 ms
    /// - 100000: 42.6 ms
    /// - 1000000: 42.6 ms
    const TRAVERSAL_LIMIT: usize = 100;

    #[builder]
    pub fn new(
        #[builder(start_fn)] pattern: &'a str,
        #[builder(default = false)] is_pattern_partial: bool,
        #[cfg(feature = "pinyin")] pinyin: Option<&'a PinyinMatchConfig<'a>>,
        #[cfg(feature = "romaji")] romaji: Option<&'a RomajiMatchConfig<'a>>,
    ) -> Self {
        // debug_assert_eq!(pattern, pattern.to_mono_lowercase());
        // TODO: Case
        Self {
            pattern,
            is_pattern_partial,
            #[cfg(feature = "pinyin")]
            pinyin,
            #[cfg(feature = "pinyin")]
            pinyin_result: Default::default(),
            #[cfg(feature = "romaji")]
            romaji,
            traversal_count: 0,
            #[cfg(test)]
            min_haystack_chars: 0,
            min_haystack_len: 0,
        }
    }

    fn analyze_default(&mut self) {
        self.analyze(PatternAnalyzeConfig::default());
    }

    #[cfg(test)]
    fn analyze_std(&mut self) {
        self.analyze(PatternAnalyzeConfig::standard());
    }

    pub fn analyze(&mut self, config: PatternAnalyzeConfig) {
        #[cfg(feature = "pinyin")]
        if let Some(pinyin) = self.pinyin {
            // TODO: Non-partial pattern
            self.pinyin_result.partial_pattern =
                self.is_pattern_partial && pinyin.allow_partial_pattern;
        }

        #[cfg(test)]
        {
            self.min_haystack_chars = usize::MAX;
        }
        self.min_haystack_len = usize::MAX;

        #[cfg(feature = "romaji")]
        if let Some(_romaji) = self.romaji {
            // KANJI_ROMAJI_MAX_LEN is 22, word is unsure, we just give up
            // TODO: traversal?
            self.set_min_haystack_chars(1);
            self.set_min_haystack_len(ib_romaji::data::MIN_LEN);
        }

        if config.traversal {
            #[cfg(feature = "pinyin")]
            {
                self.pinyin_result.used_notations = PinyinNotation::empty();
            }

            self.traversal_count = 0;
            self.sub_analyze(self.pattern, 0, 0);
            #[cfg(test)]
            println!(
                "traversal_count: {}, min_haystack_chars: {}, min_haystack_len: {}",
                self.traversal_count, self.min_haystack_chars, self.min_haystack_len
            );
            if self.traversal_count > Self::TRAVERSAL_LIMIT {
                self.analyze_default();
            }
        } else {
            #[cfg(feature = "pinyin")]
            {
                self.pinyin_result.used_notations = self
                    .pinyin
                    .map(|py| py.notations)
                    .unwrap_or(PinyinNotation::empty());
            }

            // Traversal can give a better lower bound
            #[cfg(feature = "pinyin")]
            let max_len = self.pinyin_result.used_notations.max_len();
            #[cfg(not(feature = "pinyin"))]
            let max_len = None;
            let min_haystack_chars = {
                match max_len {
                    Some(max_len) => {
                        // - Ascii: "shuang" / 6 = 1, "a" / 6 = 1
                        self.pattern.len().div_ceil(max_len)
                    }
                    None => {
                        // If case_insensitive, pattern length in bytes may be shorter than the matched haystack (or not?), so we use char count only
                        self.pattern.len()
                    }
                }
            };
            self.set_min_haystack_chars(min_haystack_chars);
            self.set_min_haystack_len(min_haystack_chars);
        }
    }

    fn sub_analyze(&mut self, pattern: &str, depth: usize, min_len: usize) {
        self.traversal_count += 1;
        if self.traversal_count > Self::TRAVERSAL_LIMIT {
            return;
        }

        if pattern.is_empty() {
            self.set_min_haystack_chars(depth);
            self.set_min_haystack_len(min_len);
            #[cfg(test)]
            if min_len < self.min_haystack_len {
                println!("{}min_haystack_len: {min_len}", " ".repeat(depth));
            }
            return;
        }
        let c = pattern.chars().next().unwrap();

        // TODO: Memoization?

        let mut any_matched_single_char = false;
        #[cfg(feature = "pinyin")]
        if let Some(pinyin) = self.pinyin {
            for notation in pinyin.notations.iter() {
                // partial:
                // Only pinyin > 4 bytes can make a difference on `min_haystack_len`,
                // i.e. chong/chuai/chuan/chuang, jiong/qiong/xiong, zhong;
                // otherwise it can't be shorter as the hanzi needs 3 bytes.
                for (py, partial) in pinyin.data.match_pinyin_partial(
                    notation,
                    pattern,
                    self.pinyin_result.partial_pattern,
                ) {
                    let matched = if partial { pattern } else { py };

                    let mut matched_single_char = false;
                    if matched.len() == 1 {
                        matched_single_char = true;

                        if notation == PinyinNotation::Ascii
                            && pinyin.notations.contains(PinyinNotation::AsciiFirstLetter)
                        {
                            // Only let AsciiFirstLetter analyze to prune the tree
                            continue;
                        }
                    } else if pinyin.notations.contains(PinyinNotation::Unicode)
                        && matched.chars().nth(1).is_none()
                    {
                        matched_single_char = true;
                    }
                    any_matched_single_char |= matched_single_char;

                    self.pinyin_result.used_notations |= notation;

                    // `MAX_RANGE` starts from 0x3007, at least 3 bytes
                    // `c.len_utf8()` for pruning the tree with `any_matched_single_char`
                    let min_len = min_len
                        + if matched_single_char {
                            min(3, c.len_utf8())
                        } else {
                            3
                        };

                    #[cfg(test)]
                    println!(
                        "{}{py} {:X} min_len={min_len} single={matched_single_char}{}",
                        " ".repeat(depth),
                        notation.bits(),
                        if partial { " partial" } else { "" }
                    );

                    self.sub_analyze(&pattern[matched.len()..], depth + 1, min_len);
                    if self.traversal_count > Self::TRAVERSAL_LIMIT {
                        return;
                    }
                }
            }
        }

        // Prune the tree
        // TODO: Optimize no_plain
        if !any_matched_single_char {
            let matched = c;
            #[cfg(test)]
            println!("{}{matched}", " ".repeat(depth));
            let len = matched.len_utf8();
            self.sub_analyze(&pattern[len..], depth + 1, min_len + len);
        }
    }

    #[cfg(feature = "pinyin")]
    pub fn pinyin(&self) -> &PinyinAnalyzeResult {
        &self.pinyin_result
    }

    fn set_min_haystack_chars(&mut self, _chars: usize) {
        #[cfg(test)]
        {
            self.min_haystack_chars = min(self.min_haystack_chars, _chars);
        }
    }

    #[cfg(test)]
    pub fn min_haystack_chars(&self) -> usize {
        self.min_haystack_chars
    }

    fn set_min_haystack_len(&mut self, len: usize) {
        self.min_haystack_len = min(self.min_haystack_len, len);
    }

    pub fn min_haystack_len(&self) -> usize {
        self.min_haystack_len
    }

    #[cfg(test)]
    fn tree(&self) {
        self.sub_tree(self.pattern, 0)
    }

    #[cfg(test)]
    fn sub_tree(&self, pattern: &str, depth: usize) {
        if pattern.is_empty() {
            return;
        }
        #[cfg(feature = "pinyin")]
        if let Some(pinyin) = self.pinyin {
            for notation in pinyin.notations.iter() {
                for matched in pinyin.data.match_pinyin(notation, pattern) {
                    println!("{}{matched} {:X}", " ".repeat(depth), notation.bits());
                    self.sub_tree(&pattern[matched.len()..], depth + 1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pinyin::PinyinData;

    use super::*;

    #[ignore]
    #[test]
    fn tree() {
        let notations = PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter;
        let pinyin_data = PinyinData::new(notations);
        let pinyin = PinyinMatchConfig::builder(notations)
            .data(&pinyin_data)
            .build();
        let pattern = "pysousuoeve";
        let analyzer = PatternAnalyzer::builder(pattern).pinyin(&pinyin).build();
        analyzer.tree();
    }

    #[test]
    fn used_notations() {
        let pinyin_data = PinyinData::new(PinyinNotation::all());
        let pinyin =
            PinyinMatchConfig::builder(PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter)
                .data(&pinyin_data)
                .build();

        let mut analyzer = PatternAnalyzer::builder("pysousuoeve")
            .pinyin(&pinyin)
            .build();
        analyzer.analyze_std();
        assert_eq!(
            analyzer.pinyin().used_notations,
            PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter
        );

        let mut analyzer = PatternAnalyzer::builder("pyssEve").pinyin(&pinyin).build();
        analyzer.analyze_std();
        assert_eq!(
            analyzer.pinyin().used_notations,
            PinyinNotation::AsciiFirstLetter
        );
    }

    #[test]
    fn min_haystack_len() {
        let pinyin_data = PinyinData::new(PinyinNotation::all());
        let pinyin =
            PinyinMatchConfig::builder(PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter)
                .data(&pinyin_data)
                .build();

        let mut analyzer = PatternAnalyzer::builder("pinyi").pinyin(&pinyin).build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 1);
        assert_eq!(analyzer.min_haystack_len(), 1);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 2);
        assert_eq!(analyzer.min_haystack_len(), 5);

        let mut analyzer = PatternAnalyzer::builder("pysousuoeve")
            .pinyin(&pinyin)
            .build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 2);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 7);
        assert_eq!(analyzer.min_haystack_len(), 11);

        let mut analyzer = PatternAnalyzer::builder("pyssEve").pinyin(&pinyin).build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 2);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 7);
        assert_eq!(analyzer.min_haystack_len(), 7);

        let mut analyzer = PatternAnalyzer::builder("pysseve").pinyin(&pinyin).build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 2);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 6);
        assert_eq!(analyzer.min_haystack_len(), 7);
    }

    #[test]
    fn min_haystack_len_partial_pattern() {
        let pinyin_data = PinyinData::new(PinyinNotation::all());
        let pinyin =
            PinyinMatchConfig::builder(PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter)
                .data(&pinyin_data)
                .build();

        let mut analyzer = PatternAnalyzer::builder("pysswe").pinyin(&pinyin).build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 1);
        assert_eq!(analyzer.min_haystack_len(), 1);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 6);
        assert_eq!(analyzer.min_haystack_len(), 6);

        let mut analyzer = PatternAnalyzer::builder("pysswe")
            .is_pattern_partial(true)
            .pinyin(&pinyin)
            .build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 1);
        assert_eq!(analyzer.min_haystack_len(), 1);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 5);
        assert_eq!(analyzer.min_haystack_len(), 6);

        let mut analyzer = PatternAnalyzer::builder("pyssyon").pinyin(&pinyin).build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 2);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 6);
        assert_eq!(analyzer.min_haystack_len(), 7);

        let mut analyzer = PatternAnalyzer::builder("pyssyon")
            .is_pattern_partial(true)
            .pinyin(&pinyin)
            .build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 2);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 5);
        assert_eq!(analyzer.min_haystack_len(), 7);

        let mut analyzer = PatternAnalyzer::builder("pyssyon").pinyin(&pinyin).build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 2);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 6);
        assert_eq!(analyzer.min_haystack_len(), 7);

        let mut analyzer = PatternAnalyzer::builder("pyssxion").pinyin(&pinyin).build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 2);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 7);
        assert_eq!(analyzer.min_haystack_len(), 8);

        let mut analyzer = PatternAnalyzer::builder("pyssxion")
            .is_pattern_partial(true)
            .pinyin(&pinyin)
            .build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 2);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 5);
        assert_eq!(analyzer.min_haystack_len(), 7);
    }

    #[test]
    fn min_haystack_len_romaji() {
        let romanizer = Default::default();
        let romaji = RomajiMatchConfig::builder().romanizer(&romanizer).build();
        let pinyin_data = PinyinData::new(PinyinNotation::all());
        let pinyin =
            PinyinMatchConfig::builder(PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter)
                .data(&pinyin_data)
                .build();

        let mut analyzer = PatternAnalyzer::builder("tsutsutsutsu")
            .romaji(&romaji)
            .build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 1);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 1);
        assert_eq!(analyzer.min_haystack_len(), 2);

        let mut analyzer = PatternAnalyzer::builder("tsutsutsutsu")
            .pinyin(&pinyin)
            .romaji(&romaji)
            .build();
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 1);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 1);
        assert_eq!(analyzer.min_haystack_len(), 2);

        let mut analyzer = PatternAnalyzer::builder("kusanomuragari")
            .pinyin(&pinyin)
            .romaji(&romaji)
            .build();
        // 丵
        analyzer.analyze_default();
        assert_eq!(analyzer.min_haystack_chars(), 1);
        assert_eq!(analyzer.min_haystack_len(), 2);
        analyzer.analyze_std();
        assert_eq!(analyzer.min_haystack_chars(), 1);
        assert_eq!(analyzer.min_haystack_len(), 2);
    }
}

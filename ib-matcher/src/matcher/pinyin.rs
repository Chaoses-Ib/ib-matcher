use std::borrow::Cow;

use bon::{bon, builder, Builder};

use crate::pinyin::{PinyinData, PinyinNotation};

/// ## Performance
/// To avoid initialization cost, you should share one `data` across all configs by either passing `&data`:
/// ```
/// use ib_matcher::{matcher::PinyinMatchConfig, pinyin::{PinyinData, PinyinNotation}};
///
/// let data = PinyinData::new(PinyinNotation::Ascii);
/// let config = PinyinMatchConfig::builder(PinyinNotation::Ascii).data(&data).build();
/// let config2 = PinyinMatchConfig::builder(PinyinNotation::Ascii).data(&data).build();
/// ```
/// Or using `shallow_clone()`:
/// ```
/// use ib_matcher::{matcher::PinyinMatchConfig, pinyin::PinyinNotation};
///
/// let config = PinyinMatchConfig::notations(PinyinNotation::Ascii);
/// let config2 = config.shallow_clone();
/// ```
#[derive(Builder, Clone)]
pub struct PinyinMatchConfig<'a> {
    #[builder(start_fn)]
    pub(crate) notations: PinyinNotation,

    /// Default: `new()` on [`PinyinMatchConfigBuilder::build()`]
    ///
    /// Must be inited with required notations if `inmut-data` feature is not enabled.
    #[builder(default = Cow::Owned(PinyinData::new(notations)))]
    #[builder(with = |data: &'a PinyinData| Cow::Borrowed(data))]
    pub(crate) data: Cow<'a, PinyinData>,

    /// Whether upper case letters can match pinyins.
    #[builder(default = false)]
    pub(crate) case_insensitive: bool,

    #[builder(default = true)]
    pub(crate) allow_partial_pattern: bool,
}

impl Default for PinyinMatchConfig<'_> {
    fn default() -> Self {
        Self::notations(PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter)
    }
}

impl<'a> PinyinMatchConfig<'a> {
    /// Use [`PinyinMatchConfigBuilder`] for more options.
    pub fn notations(notations: PinyinNotation) -> Self {
        Self::builder(notations).build()
    }

    /// See [`PinyinMatchConfig`].
    pub fn shallow_clone(&'a self) -> Self {
        Self {
            notations: self.notations,
            data: Cow::Borrowed(self.data.as_ref()),
            case_insensitive: self.case_insensitive,
            allow_partial_pattern: self.allow_partial_pattern,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinyinAnalyzeResult {
    /// - If [`PinyinNotation::Ascii`] and [`PinyinNotation::AsciiFirstLetter`] are both enabled, [`PinyinNotation::Ascii`] is only considered used if the pattern uses any non-single-letter pinyin from [`PinyinNotation::Ascii`].
    pub used_notations: PinyinNotation,
    pub partial_pattern: bool,
}

impl Default for PinyinAnalyzeResult {
    fn default() -> Self {
        Self {
            used_notations: PinyinNotation::empty(),
            partial_pattern: false,
        }
    }
}

pub(crate) struct PinyinMatcher<'a> {
    pub config: PinyinMatchConfig<'a>,
    pub notations_prefix_group: Box<[PinyinNotation]>,
    pub notations: Box<[PinyinNotation]>,
    pub partial_pattern: bool,
}

#[bon]
impl<'a> PinyinMatcher<'a> {
    pub const ORDERED_PINYIN_NOTATIONS: [PinyinNotation; 10] = [
        PinyinNotation::AsciiFirstLetter,
        PinyinNotation::Ascii,
        PinyinNotation::AsciiTone,
        PinyinNotation::Unicode,
        PinyinNotation::DiletterAbc,
        PinyinNotation::DiletterJiajia,
        PinyinNotation::DiletterMicrosoft,
        PinyinNotation::DiletterThunisoft,
        PinyinNotation::DiletterXiaohe,
        PinyinNotation::DiletterZrm,
    ];

    #[builder]
    pub fn new(
        #[builder(start_fn)] config: PinyinMatchConfig<'a>,
        analyze: PinyinAnalyzeResult,
    ) -> Self {
        let used_notations = analyze.used_notations;

        let (notations_prefix_group, unprefixable_notations) = match used_notations
            .intersection(
                PinyinNotation::AsciiFirstLetter
                    | PinyinNotation::Ascii
                    | PinyinNotation::AsciiTone,
            )
            .bits()
            .count_ones()
        {
            count if count > 1 => {
                let mut notations = Vec::with_capacity(count as usize);
                if used_notations.contains(PinyinNotation::AsciiFirstLetter) {
                    notations.push(PinyinNotation::AsciiFirstLetter);
                }
                if used_notations.contains(PinyinNotation::Ascii) {
                    notations.push(PinyinNotation::Ascii);
                }
                if used_notations.contains(PinyinNotation::AsciiTone) {
                    notations.push(PinyinNotation::AsciiTone);
                }
                (
                    notations,
                    used_notations.difference(
                        PinyinNotation::AsciiFirstLetter
                            | PinyinNotation::Ascii
                            | PinyinNotation::AsciiTone,
                    ),
                )
            }
            _ => (Vec::new(), used_notations),
        };
        let mut notations = Vec::with_capacity(unprefixable_notations.bits().count_ones() as usize);
        for notation in Self::ORDERED_PINYIN_NOTATIONS {
            if unprefixable_notations.contains(notation) {
                notations.push(notation);
            }
        }

        Self {
            partial_pattern: analyze.partial_pattern,
            notations_prefix_group: notations_prefix_group.into_boxed_slice(),
            notations: notations.into_boxed_slice(),
            config,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{assert_match, matcher::IbMatcher};

    use super::*;

    #[test]
    fn ordered_notations() {
        assert_eq!(
            PinyinNotation::all().iter().count(),
            PinyinMatcher::ORDERED_PINYIN_NOTATIONS.len()
        )
    }

    #[test]
    fn diletter() {
        // rs tw he ne nt er fo ld er
        let m = IbMatcher::builder("Event.SelectFirstWhenEnterFolder.js")
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::DiletterXiaohe))
            .analyze(true)
            .build();
        assert_match!(m.test("Event.SelectFirstWhenEnterFolder.js"), Some((0, 35)));

        let m = IbMatcher::builder("Event.SelectFirstWhenEnterFolder.js")
            .pinyin(PinyinMatchConfig::notations(
                PinyinNotation::Ascii
                    | PinyinNotation::AsciiFirstLetter
                    | PinyinNotation::DiletterXiaohe,
            ))
            .analyze(true)
            .build();
        assert_match!(m.test("Event.SelectFirstWhenEnterFolder.js"), Some((0, 35)));

        let m = IbMatcher::builder("Event.SelectFirstWhenEnterFolder.js")
            .is_pattern_partial(true)
            .pinyin(PinyinMatchConfig::notations(
                PinyinNotation::Ascii
                    | PinyinNotation::AsciiFirstLetter
                    | PinyinNotation::DiletterXiaohe,
            ))
            .analyze(true)
            .build();
        assert_match!(m.test("Event.SelectFirstWhenEnterFolder.js"), Some((0, 35)));
    }
}

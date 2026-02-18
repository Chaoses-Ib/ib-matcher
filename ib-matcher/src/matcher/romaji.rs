use std::borrow::Cow;

use bon::Builder;
use ib_romaji::HepburnRomanizer;

/// ## Performance
/// To avoid initialization cost, you should share one `romanizer` across all configs by either passing `&romanizer`:
/// ```
/// use ib_matcher::{matcher::RomajiMatchConfig, romaji::HepburnRomanizer};
///
/// let romanizer = HepburnRomanizer::default();
/// let config = RomajiMatchConfig::builder().romanizer(&romanizer).build();
/// let config2 = RomajiMatchConfig::builder().romanizer(&romanizer).build();
/// ```
/// Or using `shallow_clone()`:
/// ```
/// use ib_matcher::matcher::RomajiMatchConfig;
///
/// let config = RomajiMatchConfig::default();
/// let config2 = config.shallow_clone();
/// ```
#[derive(Builder, Clone)]
pub struct RomajiMatchConfig<'a> {
    /// Default: `new()` on [`RomajiMatchConfigBuilder::build()`]
    #[builder(default = Cow::Owned(HepburnRomanizer::default()))]
    #[builder(with = |romanizer: &'a HepburnRomanizer| Cow::Borrowed(romanizer))]
    pub(crate) romanizer: Cow<'a, HepburnRomanizer>,

    /// Whether upper case letters can match Japanese words.
    #[builder(default = false)]
    pub(crate) case_insensitive: bool,

    #[builder(default = true)]
    pub(crate) allow_partial_pattern: bool,
}

impl Default for RomajiMatchConfig<'_> {
    /// Use [`RomajiMatchConfigBuilder`] for more options.
    fn default() -> Self {
        Self::builder().build()
    }
}

impl<'a> RomajiMatchConfig<'a> {
    /// See [`RomajiMatchConfig`].
    pub fn shallow_clone(&'a self) -> RomajiMatchConfig<'a> {
        Self {
            romanizer: Cow::Borrowed(self.romanizer.as_ref()),
            case_insensitive: self.case_insensitive,
            allow_partial_pattern: self.allow_partial_pattern,
        }
    }
}

pub(crate) struct RomajiMatcher<'a> {
    pub config: RomajiMatchConfig<'a>,
    pub partial_pattern: bool,
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_match,
        matcher::{IbMatcher, MatchConfig},
    };

    use super::*;

    #[test]
    fn romaji() {
        let romanizer = Default::default();
        let romaji = RomajiMatchConfig::builder().romanizer(&romanizer).build();

        let matcher = IbMatcher::builder("ohayo").romaji(romaji.clone()).build();
        assert_match!(matcher.find("おはよう"), Some((0, 9)));

        let matcher = IbMatcher::builder("jojo").romaji(romaji.clone()).build();
        assert_match!(matcher.find("おはよジョジョ"), Some((9, 12)));

        let matcher = IbMatcher::builder("konosubarashiisekaini")
            .romaji(romaji.clone())
            .build();
        assert_match!(matcher.find("この素晴らしい世界に祝福を"), Some((0, 30)));

        let matcher = IbMatcher::builder("konosuba")
            .romaji(romaji.clone())
            .build();
        assert_match!(matcher.find("この素晴らしい世界に祝福を"), None);
        let matcher = IbMatcher::builder("konosuba")
            .romaji(romaji.clone())
            .is_pattern_partial(true)
            .build();
        assert_match!(
            matcher.find("この素晴らしい世界に祝福を"),
            Some((0, 21)),
            partial
        );
    }

    #[test]
    fn n_apostrophe() {
        let config = MatchConfig::builder()
            .romaji(Default::default())
            .starts_with(true)
            .build();
        let m = IbMatcher::with_config("kan", config.shallow_clone());
        assert_match!(m.find("かん"), Some((0, 6)));
        assert_match!(m.find("かに"), None);

        let m = IbMatcher::with_config("kann", config.shallow_clone());
        assert_match!(m.find("かんん"), Some((0, 9)));
        assert_match!(m.find("かんに"), None);

        let m = IbMatcher::with_config("kann'", config.shallow_clone());
        // ' suffix is neither supported nor needed
        assert_match!(m.find("かんん"), None);
        assert_match!(m.find("かんに"), None);

        let m = IbMatcher::with_config("kanni", config.shallow_clone());
        // Unfortunately, in IME using "nn", this will be かんい
        assert_match!(m.find("かんに"), Some((0, 9)));
        assert_match!(m.find("かんんい"), None);

        let m = IbMatcher::with_config("kann'i", config.shallow_clone());
        assert_match!(m.find("かんに"), None);
        assert_match!(m.find("かんんい"), Some((0, 12)));

        let m = IbMatcher::with_config("botan'yuki", config.shallow_clone());
        assert_match!(m.find("ボタン雪"), Some((0, 12)));
    }

    #[test]
    fn n_apostrophe_partial() {
        let config = MatchConfig::builder()
            .romaji(Default::default())
            .starts_with(true)
            .is_pattern_partial(true)
            .build();
        let m = IbMatcher::with_config("kan", config.shallow_clone());
        assert_match!(m.find("かん"), Some((0, 6)));
        assert_match!(m.find("かに"), Some((0, 6)), partial);

        let m = IbMatcher::with_config("kann", config.shallow_clone());
        assert_match!(m.find("かんん"), Some((0, 9)));
        assert_match!(m.find("かんに"), Some((0, 9)), partial);

        let m = IbMatcher::with_config("kann'", config.shallow_clone());
        assert_match!(m.find("かんん"), None);
        assert_match!(m.find("かんに"), None);

        let m = IbMatcher::with_config("kanni", config.shallow_clone());
        assert_match!(m.find("かんに"), Some((0, 9)));
        assert_match!(m.find("かんんい"), None);

        let m = IbMatcher::with_config("kann'i", config.shallow_clone());
        assert_match!(m.find("かんに"), None);
        assert_match!(m.find("かんんい"), Some((0, 12)));
    }

    #[test]
    fn kanji_noma() {
        let config = MatchConfig::builder()
            .romaji(Default::default())
            .starts_with(true)
            .build();

        let m = IbMatcher::with_config("mizukina", config.shallow_clone());
        assert_match!(m.find("水樹奈々"), Some((0, 9)));
        let m = IbMatcher::with_config("mizukinana", config.shallow_clone());
        assert_match!(m.find("水樹奈々"), Some((0, 12)));
    }

    #[test]
    fn min_haystack_len() {
        let romanizer = Default::default();
        let romaji = RomajiMatchConfig::builder().romanizer(&romanizer).build();

        let matcher = IbMatcher::builder("kusanomuragari")
            .romaji(romaji.clone())
            .build();
        assert_match!(matcher.test("丵"), Some((0, 3)));

        let matcher = IbMatcher::builder("suraritoshitemimeyoi")
            .romaji(romaji.clone())
            .build();
        assert_match!(matcher.test("娍"), Some((0, 3)));

        let matcher =
            IbMatcher::builder("shintaihappukorewofuboniukuaetekishousezaruhakounohajimenari")
                .romaji(romaji.clone())
                .build();
        assert_match!(
            matcher.test("身体髪膚これを父母に受くあえて毀傷せざるは孝の始めなり"),
            Some((0, 81))
        );
    }
}

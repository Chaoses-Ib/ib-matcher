/*!
A collection of syntax parsers for either [`IbMatcher`](crate::matcher::IbMatcher) or [`Regex`](crate::regex::cp::Regex).

## glob()-style pattern matching syntax
See [`glob`] for details. For example:
```
use ib_matcher::{regex::cp::Regex, syntax::glob::{parse_wildcard_path, PathSeparator}};

let re = Regex::builder()
    .build_from_hir(
        parse_wildcard_path()
            .separator(PathSeparator::Windows)
            .call(r"Win*\*\*.exe"),
    )
    .unwrap();
assert!(re.is_match(r"C:\Windows\System32\notepad.exe"));
```

## IbEverythingExt flavour
*/
//! Parse a pattern according to the syntax used by [IbEverythingExt](https://github.com/Chaoses-Ib/IbEverythingExt).
//!
//! See [`Pattern::parse_ev`].
//!
//! ### Example
//! ```
//! use ib_matcher::{matcher::{IbMatcher, PinyinMatchConfig, pattern::Pattern}, pinyin::PinyinNotation};
//!
//! let matcher = IbMatcher::builder(Pattern::parse_ev("pinyin;py").call())
//!     .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
//!     .build();
//! assert!(matcher.is_match("拼音搜索"));
//! assert!(matcher.is_match("pinyin") == false);
//! ```
use bon::bon;

use crate::matcher::pattern::{LangOnly, Pattern};

#[cfg(feature = "syntax-glob")]
pub mod glob;

#[cfg(feature = "syntax")]
#[bon]
impl<'a> Pattern<'a, str> {
    /// Parse a pattern according to the syntax used by [IbEverythingExt](https://github.com/Chaoses-Ib/IbEverythingExt).
    ///
    /// - `;en`, `;py` and `;rm` postmodifiers are mutually exclusive. If multiple are present, only the last one will be considered as a postmodifier.
    ///
    /// Only UTF-8 pattern is supported at the moment.
    ///
    /// ## Example
    /// ```
    /// use ib_matcher::{matcher::{IbMatcher, PinyinMatchConfig, pattern::Pattern}, pinyin::PinyinNotation};
    ///
    /// let matcher = IbMatcher::builder(Pattern::parse_ev("pinyin;py").call())
    ///     .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
    ///     .build();
    /// assert!(matcher.is_match("拼音搜索"));
    /// assert!(matcher.is_match("pinyin") == false);
    /// ```
    #[builder]
    pub fn parse_ev(
        #[builder(start_fn)] mut pattern: &'a str,

        /// `;en` (English) postmodifier that disables both pinyin and romaji match, if any.
        #[builder(default = true)]
        postmodifier_en: bool,
        /// `;py` (pinyin) postmodifier that indicates the pattern should be matched as pinyin only.
        #[builder(default = true)]
        postmodifier_py: bool,
        /// `;rm` (romaji) postmodifier that indicates the pattern should be matched as romaji only.
        #[builder(default = true)]
        postmodifier_rm: bool,
    ) -> Self {
        let mut lang_only = None;
        if let Some(s) = pattern.strip_suffix(";en").filter(|_| postmodifier_en) {
            lang_only = Some(LangOnly::English);
            pattern = s;
        } else if let Some(s) = pattern.strip_suffix(";py").filter(|_| postmodifier_py) {
            lang_only = Some(LangOnly::Pinyin);
            pattern = s;
        } else if let Some(s) = pattern.strip_suffix(";rm").filter(|_| postmodifier_rm) {
            lang_only = Some(LangOnly::Romaji);
            pattern = s;
        }

        Self { pattern, lang_only }
    }
}

// #[bon]
// impl<'a, 'f1, HaystackStr, S: ib_matcher_builder::State> IbMatcherBuilder<'a, 'f1, HaystackStr, S>
// where
//     HaystackStr: EncodedStr + ?Sized,
// {
//     #[builder(finish_fn = build)]
//     pub fn parse_ev(self, case_insensitive: bool) -> IbMatcher<'a, HaystackStr>
//     where
//         S: ib_matcher_builder::IsComplete,
//     {
//         dbg!(&self.pattern.as_bytes());
//         self.build()
//     }
// }

#[cfg(test)]
mod tests {
    use crate::{
        matcher::{IbMatcher, PinyinMatchConfig},
        pinyin::PinyinNotation,
    };

    use super::*;

    #[test]
    fn lang_only() {
        let p = Pattern::parse_ev("pinyin").call();
        assert!(p.lang_only.is_none());

        let p = Pattern::parse_ev("pinyin;en").call();
        assert_eq!(p.lang_only, Some(LangOnly::English));

        let p = Pattern::parse_ev("pinyin;py").call();
        assert_eq!(p.lang_only, Some(LangOnly::Pinyin));

        let p = Pattern::parse_ev("pinyin;rm").call();
        assert_eq!(p.lang_only, Some(LangOnly::Romaji));

        let p = Pattern::parse_ev("pinyin;en;py").call();
        assert_eq!(p.pattern, "pinyin;en");
        assert_eq!(p.lang_only, Some(LangOnly::Pinyin));

        let matcher = IbMatcher::builder(Pattern::parse_ev("pinyin;py").call())
            .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
            .build();
        assert!(matcher.is_match("拼音搜索"));
        assert!(matcher.is_match("pinyin") == false);
    }
}

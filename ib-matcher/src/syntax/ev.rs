/*!
# IbEverythingExt flavour
Parse a pattern according to the syntax used by [IbEverythingExt](https://github.com/Chaoses-Ib/IbEverythingExt).

See [`Pattern::parse_ev`].

## Example
```
// cargo add ib-matcher --features syntax-ev,pinyin
use ib_matcher::{matcher::{IbMatcher, PinyinMatchConfig, pattern::Pattern}, pinyin::PinyinNotation};

let matcher = IbMatcher::builder(Pattern::parse_ev("pinyin;py").call())
    .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
    .build();
assert!(matcher.is_match("拼音搜索"));
assert!(matcher.is_match("pinyin") == false);
```

## With `Regex`
```
use ib_matcher::{regex::cp::Regex, matcher::{MatchConfig, pattern::Pattern}};

let re = Regex::builder()
    .ib(MatchConfig::builder().pinyin(Default::default()).build())
    .ib_parser(&mut |pattern| Pattern::parse_ev(pattern).call())
    .build("pinyin;py")
    .unwrap();
assert!(re.is_match("拼音搜索"));
assert!(re.is_match("pinyin") == false);
```

## With [`glob`](super::glob)
```
use ib_matcher::{
    matcher::{MatchConfig, pattern::Pattern},
    regex::cp::Regex,
    syntax::glob::{parse_wildcard_path, PathSeparator}
};

let re = Regex::builder()
    .ib(MatchConfig::builder().pinyin(Default::default()).build())
    .ib_parser(&mut |pattern| Pattern::parse_ev(pattern).call())
    .build_from_hir(
        parse_wildcard_path()
            .separator(PathSeparator::Windows)
            .call(r"pinyin;py**sou;py"),
    )
    .unwrap();
assert!(re.is_match(r"C:\拼音\System32\搜索.exe"));
assert!(re.is_match(r"C:\pinyin\System32\搜索.exe") == false);
assert!(re.is_match(r"C:\pinyin\System32\sousuo.exe") == false);
```
*/

use bon::bon;

use crate::matcher::pattern::{LangOnly, Pattern};

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
        matcher::{IbMatcher, MatchConfig, PinyinMatchConfig},
        pinyin::PinyinNotation,
        regex::cp::Regex,
        syntax::glob::{parse_wildcard_path, PathSeparator},
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

    #[test]
    fn re() {
        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .ib_parser(&mut |pattern| Pattern::parse_ev(pattern).call())
            .build("pinyin;py")
            .unwrap();
        assert!(re.is_match("拼音搜索"));
        assert!(re.is_match("pinyin") == false);
    }

    #[test]
    fn glob() {
        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .ib_parser(&mut |pattern| Pattern::parse_ev(pattern).call())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"pinyin;py**sou;py"),
            )
            .unwrap();
        assert!(re.is_match(r"C:\拼音\System32\搜索.exe"));
        assert!(re.is_match(r"C:\pinyin\System32\搜索.exe") == false);
        assert!(re.is_match(r"C:\pinyin\System32\sousuo.exe") == false);
    }
}

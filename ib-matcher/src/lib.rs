/*!
A multilingual, flexible and fast string and regex matcher, supports 拼音匹配 (Chinese pinyin match) and ローマ字検索 (Japanese romaji match).

## Features
- Unicode support
  - Fully UTF-8 support and limited support for UTF-16 and UTF-32.
  - Unicode case insensitivity.
- [Chinese pinyin](https://en.wikipedia.org/wiki/Pinyin) matching (拼音匹配)
  - Support characters with multiple readings (i.e. heteronyms, 多音字).
  - Support multiple pinyin notations, including [Quanpin (全拼)](https://zh.wikipedia.org/wiki/全拼), [Jianpin (简拼)](https://zh.wikipedia.org/wiki/简拼) and many [Shuangpin (双拼)](https://zh.wikipedia.org/wiki/%E5%8F%8C%E6%8B%BC) notations.
  - Support mixing multiple notations during matching.
- [Japanese romaji](https://en.wikipedia.org/wiki/Romanization_of_Japanese) matching (ローマ字検索)
  - Support characters with multiple readings (i.e. heteronyms, 同形異音語).
  - Support [Hepburn romanization system](https://en.wikipedia.org/wiki/Hepburn_romanization) only at the moment.
- [glob()-style](syntax::glob) pattern matching (i.e. `?`, `*` and `**`)
- [Regular expression](regex)
  - Support the same syntax as [`regex`](https://docs.rs/regex/), including wildcards, repetitions, alternations, groups, etc.
  - Support [custom matching callbacks](regex::cp::Regex#custom-matching-callbacks), which can be used to implement ad hoc look-around, backreferences, balancing groups/recursion/subroutines, combining domain-specific parsers, etc.
- Relatively high performance

And all of the above features are optional. You don't need to pay the performance and binary size cost for features you don't use.

You can also use [ib-pinyin](https://docs.rs/ib-pinyin/) if you only need Chinese pinyin match, which is simpler and more stable.
*/
//! ## Usage
//! ```
//! //! cargo add ib-matcher --features pinyin,romaji
//! use ib_matcher::{
//!     matcher::{IbMatcher, PinyinMatchConfig, RomajiMatchConfig},
//!     pinyin::PinyinNotation,
//! };
//!
//! let matcher = IbMatcher::builder("pysousuoeve")
//!     .pinyin(PinyinMatchConfig::notations(
//!         PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
//!     ))
//!     .build();
//! assert!(matcher.is_match("拼音搜索Everything"));
//!
//! let matcher = IbMatcher::builder("konosuba")
//!     .romaji(RomajiMatchConfig::default())
//!     .is_pattern_partial(true)
//!     .build();
//! assert!(matcher.is_match("この素晴らしい世界に祝福を"));
//! ```
/*!
## Regular expression
See [`regex`] module for more details. For example:
```
// cargo add ib-matcher --features regex,pinyin,romaji
use ib_matcher::{
    matcher::{MatchConfig, PinyinMatchConfig, RomajiMatchConfig},
    regex::{cp::Regex, Match},
};

let config = MatchConfig::builder()
    .pinyin(PinyinMatchConfig::default())
    .romaji(RomajiMatchConfig::default())
    .build();

let re = Regex::builder()
    .ib(config.shallow_clone())
    .build("raki.suta")
    .unwrap();
assert_eq!(re.find("「らき☆すた」"), Some(Match::must(0, 3..18)));

let re = Regex::builder()
    .ib(config.shallow_clone())
    .build("pysou.*?(any|every)thing")
    .unwrap();
assert_eq!(re.find("拼音搜索Everything"), Some(Match::must(0, 0..22)));

let config = MatchConfig::builder()
    .pinyin(PinyinMatchConfig::default())
    .romaji(RomajiMatchConfig::default())
    .mix_lang(true)
    .build();
let re = Regex::builder()
    .ib(config.shallow_clone())
    .build("(?x)^zangsounofuri-?ren # Mixing pinyin and romaji")
    .unwrap();
assert_eq!(re.find("葬送のフリーレン"), Some(Match::must(0, 0..24)));
```

[Custom matching callbacks](regex::cp::Regex#custom-matching-callbacks):
```
// cargo add ib-matcher --features regex,regex-callback
use ib_matcher::regex::cp::Regex;

let re = Regex::builder()
    .callback("ascii", |input, at, push| {
        let haystack = &input.haystack()[at..];
        if haystack.len() > 0 && haystack[0].is_ascii() {
            push(1);
        }
    })
    .build(r"(ascii)+\d(ascii)+")
    .unwrap();
let hay = "that4Ｕ this4me";
assert_eq!(&hay[re.find(hay).unwrap().span()], " this4me");
```
*/
//! ## Performance
//! The following `Cargo.toml` settings are recommended if best performance is desired:
//! ```toml
//! [profile.release]
//! lto = "fat"
//! codegen-units = 1
//! ```
//! These can improve the performance by 5~10% at most.
//!
//! ## Crate features
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(feature = "doc", doc = document_features::document_features!())]

extern crate alloc;

pub mod matcher;
#[cfg(feature = "minimal")]
pub mod minimal;
#[cfg(feature = "pinyin")]
pub mod pinyin;
#[cfg(any(feature = "regex-automata", feature = "regex-syntax"))]
pub mod regex;
#[cfg(any(feature = "syntax-glob", feature = "syntax-ev"))]
pub mod syntax;
pub mod unicode;

#[cfg(feature = "romaji")]
pub use ib_romaji as romaji;

mod private {
    pub trait Sealed {}
}
use private::Sealed;

#[cfg(test)]
mod tests {
    use crate::{
        matcher::{MatchConfig, PinyinMatchConfig, RomajiMatchConfig},
        regex::{cp::Regex, Match},
    };

    #[test]
    fn regex() {
        let config = MatchConfig::builder()
            .pinyin(PinyinMatchConfig::default())
            .romaji(RomajiMatchConfig::default())
            .build();

        let re = Regex::builder()
            .ib(config.shallow_clone())
            .build("raki.suta")
            .unwrap();
        assert_eq!(re.find("「らき☆すた」"), Some(Match::must(0, 3..18)));

        let re = Regex::builder()
            .ib(config.shallow_clone())
            .build("pysou.*?(any|every)thing")
            .unwrap();
        assert_eq!(re.find("拼音搜索Everything"), Some(Match::must(0, 0..22)));

        let config = MatchConfig::builder()
            .pinyin(PinyinMatchConfig::default())
            .romaji(RomajiMatchConfig::default())
            .mix_lang(true)
            .build();
        let re = Regex::builder()
            .ib(config.shallow_clone())
            .build("(?x)^zangsounofuri-?ren # Mixing pinyin and romaji")
            .unwrap();
        assert_eq!(re.find("葬送のフリーレン"), Some(Match::must(0, 0..24)));
    }
}

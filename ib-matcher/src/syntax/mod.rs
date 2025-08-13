/*!
A collection of syntax parsers for either [`IbMatcher`](crate::matcher::IbMatcher) or [`Regex`](crate::regex::cp::Regex).

## glob()-style pattern matching syntax
See [`glob`] for details. For example:
```
// cargo add ib-matcher --features syntax-glob,regex
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
Parse a pattern according to the syntax used by [IbEverythingExt](https://github.com/Chaoses-Ib/IbEverythingExt).

See [`ev`] for details.

### Example
```
// cargo add ib-matcher --features syntax-ev,pinyin
use ib_matcher::{matcher::{IbMatcher, PinyinMatchConfig, pattern::Pattern}, pinyin::PinyinNotation};

let matcher = IbMatcher::builder(Pattern::parse_ev("pinyin;py").call())
    .pinyin(PinyinMatchConfig::notations(PinyinNotation::Ascii))
    .build();
assert!(matcher.is_match("拼音搜索"));
assert!(matcher.is_match("pinyin") == false);
```
*/

#[cfg(feature = "syntax-glob")]
pub mod glob;

#[cfg(feature = "syntax-ev")]
pub mod ev;

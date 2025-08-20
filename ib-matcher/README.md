# ib-matcher
[![crates.io](https://img.shields.io/crates/v/ib-matcher.svg)](https://crates.io/crates/ib-matcher)
[![Documentation](https://docs.rs/ib-matcher/badge.svg)](https://docs.rs/ib-matcher)
[![License](https://img.shields.io/crates/l/ib-matcher.svg)](LICENSE.txt)

A multilingual, flexible and fast string and regex matcher, supports 拼音匹配 (Chinese pinyin match) and ローマ字検索 (Japanese romaji match).

## Features
- Unicode support
  - Fully UTF-8 support and limited support for UTF-16 and UTF-32.
  - Unicode case insensitivity ([simple case folding](https://docs.rs/ib-unicode/latest/ib_unicode/case/#case-folding)).
- [Chinese pinyin](https://en.wikipedia.org/wiki/Pinyin) matching (拼音匹配)
  - Support characters with multiple readings (i.e. heteronyms, 多音字).
  - Support multiple pinyin notations, including [Quanpin (全拼)](https://zh.wikipedia.org/wiki/全拼), [Jianpin (简拼)](https://zh.wikipedia.org/wiki/简拼) and many [Shuangpin (双拼)](https://zh.wikipedia.org/wiki/%E5%8F%8C%E6%8B%BC) notations.
  - Support mixing multiple notations during matching.
- [Japanese romaji](https://en.wikipedia.org/wiki/Romanization_of_Japanese) matching (ローマ字検索)
  - Support characters with multiple readings (i.e. heteronyms, 同形異音語).
  - Support [Hepburn romanization system](https://en.wikipedia.org/wiki/Hepburn_romanization) only at the moment.
- [glob()-style](https://docs.rs/ib-matcher/latest/ib_matcher/syntax/glob/) pattern matching (i.e. `?`, `*`, `[]` and `**`)
  - Support [different anchor modes](https://docs.rs/ib-matcher/latest/ib_matcher/syntax/glob/#anchor-modes), [treating surrounding wildcards as anchors](https://docs.rs/ib-matcher/latest/ib_matcher/syntax/glob/#surrounding-wildcards-as-anchors) and [special anchors in file paths](https://docs.rs/ib-matcher/latest/ib_matcher/syntax/glob/#anchors-in-file-paths).
  - Support two seperators (`//`) or a complement separator (`\`) as a glob star (`*/**`).
- [Regular expression](https://docs.rs/ib-matcher/latest/ib_matcher/regex/)
  - Support the same syntax as [`regex`](https://docs.rs/regex/), including wildcards, repetitions, alternations, groups, etc.
  - Support [custom matching callbacks](https://docs.rs/ib-matcher/latest/ib_matcher/regex/cp/struct.Regex.html#custom-matching-callbacks), which can be used to implement ad hoc look-around, backreferences, balancing groups/recursion/subroutines, combining domain-specific parsers, etc.
- Relatively high performance

And all of the above features are optional. You don't need to pay the performance and binary size cost for features you don't use.

You can also use [ib-pinyin](../README.md#ib-pinyin) if you only need Chinese pinyin match, which is simpler and more stable.

## Usage
```rust
//! cargo add ib-matcher --features pinyin,romaji

use ib_matcher::{
    matcher::{IbMatcher, PinyinMatchConfig, RomajiMatchConfig},
    pinyin::PinyinNotation,
};

let matcher = IbMatcher::builder("pysousuoeve")
    .pinyin(PinyinMatchConfig::notations(
        PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
    ))
    .build();
assert!(matcher.is_match("拼音搜索Everything"));

let matcher = IbMatcher::builder("konosuba")
    .romaji(RomajiMatchConfig::default())
    .is_pattern_partial(true)
    .build();
assert!(matcher.is_match("この素晴らしい世界に祝福を"));
```

## Regular expression
See [`regex` module](https://docs.rs/ib-matcher/latest/ib_matcher/regex/) for more details. For example:
```rust
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

[Custom matching callbacks](https://docs.rs/ib-matcher/latest/ib_matcher/regex/cp/struct.Regex.html#custom-matching-callbacks):
```rust
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

## Test
```sh
cargo build
cargo test --features pinyin,romaji
```

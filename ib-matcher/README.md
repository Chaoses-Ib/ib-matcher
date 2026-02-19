# ib-matcher
[![crates.io](https://img.shields.io/crates/v/ib-matcher.svg)](https://crates.io/crates/ib-matcher)
[![Documentation](https://docs.rs/ib-matcher/badge.svg)](https://docs.rs/ib-matcher)
[![License](https://img.shields.io/crates/l/ib-matcher.svg)](LICENSE.txt)

A multilingual, flexible and fast string, glob and regex matcher. Support 拼音匹配 (Chinese pinyin match) and ローマ字検索 (Japanese romaji match).

## Features
- Unicode support
  - Fully UTF-8 support and limited support for UTF-16 and UTF-32.
  - Unicode case insensitivity ([simple case folding](https://docs.rs/ib-unicode/latest/ib_unicode/case/#case-folding)).
- [Chinese pinyin](../README.md#ib-pinyin) matching (拼音匹配)
  - Support characters with multiple readings (i.e. heteronyms, 多音字).
  - Support multiple pinyin notations, including [Quanpin (全拼)](https://zh.wikipedia.org/wiki/全拼), [Jianpin (简拼)](https://zh.wikipedia.org/wiki/简拼) and many [Shuangpin (双拼)](https://zh.wikipedia.org/wiki/%E5%8F%8C%E6%8B%BC) notations.
  - Support mixing multiple notations during matching.
- [Japanese romaji](../README.md#ib-romaji) matching (ローマ字検索)
  - Support characters with multiple readings (i.e. heteronyms, 同形異音語).
  - Support [Hepburn romanization system](https://en.wikipedia.org/wiki/Hepburn_romanization)
    and its [convenient IME variant](https://docs.rs/ib-romaji/latest/ib_romaji/convert/hepburn_ime/).
  - Support handling of `n'`/`nn` and [`々`](https://docs.rs/ib-romaji/latest/ib_romaji/kanji/#handling-of-々noma).
- [glob()-style](https://docs.rs/ib-matcher/latest/ib_matcher/syntax/glob/) pattern matching (i.e. `?`, `*`, `[]` and `**`)
  - Support [different anchor modes](https://docs.rs/ib-matcher/latest/ib_matcher/syntax/glob/#anchor-modes), [treating surrounding wildcards as anchors](https://docs.rs/ib-matcher/latest/ib_matcher/syntax/glob/#surrounding-wildcards-as-anchors) and [special anchors in file paths](https://docs.rs/ib-matcher/latest/ib_matcher/syntax/glob/#anchors-in-file-paths).
  - Support two seperators (`//`) or a complement separator (`\`) as a glob star (`*/**`).
- [Regular expression](https://docs.rs/ib-matcher/latest/ib_matcher/regex/)
  - Support the same syntax as [`regex`](https://docs.rs/regex/), including wildcards, repetitions, alternations, groups, etc.
  - Support [custom matching callbacks](https://docs.rs/ib-matcher/latest/ib_matcher/regex/cp/struct.Regex.html#custom-matching-callbacks), which can be used to implement ad hoc look-around, backreferences, balancing groups/recursion/subroutines, combining domain-specific parsers, etc.
- Relatively high performance
  - Generally on par with the `regex` crate, depending on the case it can be faster or slower.

And all of the above features are optional. You don't need to pay the performance and binary size cost for features you don't use.

See [documentation](https://docs.rs/ib-matcher) for details.

You can also use [ib-pinyin](../README.md#ib-pinyin) if you only need Chinese pinyin match, which is simpler and more stable.

## Usage
```rust
// cargo add ib-matcher --features pinyin,romaji
use ib_matcher::matcher::{IbMatcher, PinyinMatchConfig, RomajiMatchConfig};

let matcher = IbMatcher::builder("la vie est drôle").build();
assert!(matcher.is_match("LA VIE EST DRÔLE"));

let matcher = IbMatcher::builder("βίος").build();
assert!(matcher.is_match("Βίοσ"));
assert!(matcher.is_match("ΒΊΟΣ"));

let matcher = IbMatcher::builder("pysousuoeve")
    .pinyin(PinyinMatchConfig::default())
    .build();
assert!(matcher.is_match("拼音搜索Everything"));

let matcher = IbMatcher::builder("konosuba")
    .romaji(RomajiMatchConfig::default())
    .build();
assert!(matcher.is_match("『この素晴らしい世界に祝福を』"));
// Matching is unanchored by default, you can set `b.starts_with(true)` for anchored one.
```

`MatchConfig` and Japanese romaji matching examples:
```rust
// cargo add ib-matcher --features romaji,macros
use ib_matcher::{assert_match, matcher::MatchConfig};

let c = MatchConfig::builder().romaji(Default::default()).build();
// kya n
assert_match!(c.matcher("kyan").find("キャン"), Some((0, 9)));
// kya ni
assert_match!(c.matcher("kyan").find("キャニ"), None);
// Partial match (`b.is_pattern_partial()`) is disabled by default.

// kya n(n'/nn) i se kai nyo nyo
assert_match!(c.matcher("nisekainyonyo" ).find("キャンヰ世界ニョニョ"), None);
assert_match!(c.matcher("n'isekainyonyo").find("キャンヰ世界ニョニョ"), Some((6, 24)));
assert_match!(c.matcher("nnisekainyonyo").find("キャンヰ世界ﾆｮﾆｮ"   ), Some((6, 24)));

// shu u se i pa tchi/cchi
assert_match!(c.matcher("shuuseipatchi").find("修正パッチ"), Some((0, 15)));
assert_match!(c.matcher("shuuseipacchi").find("集成パッチ"), Some((0, 15)));

// shi ka no ko no ko no ko ko shi ta n ta n
assert_match!(c.matcher("shikanokonokonokokoshitantan").find("鹿乃子のこのこ虎視眈々"), Some((0, 33)));
```

## glob()-style pattern matching
See [`glob` module](https://docs.rs/ib-matcher/latest/ib_matcher/syntax/glob/) for more details. Here is a quick example:
```rust
// cargo add ib-matcher --features syntax-glob,regex,romaji
use ib_matcher::{
    matcher::MatchConfig,
    regex::lita::Regex,
    syntax::glob::{parse_wildcard_path, PathSeparator}
};

let re = Regex::builder()
    .ib(MatchConfig::builder().romaji(Default::default()).build())
    .build_from_hir(
        parse_wildcard_path()
            .separator(PathSeparator::Windows)
            .call("wifi**miku"),
    )
    .unwrap();
assert!(re.is_match(r"C:\Windows\System32\ja-jp\WiFiTask\ミク.exe"));
```

## Regular expression
See [`regex` module](https://docs.rs/ib-matcher/latest/ib_matcher/regex/) for more details. Here is a quick example:
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

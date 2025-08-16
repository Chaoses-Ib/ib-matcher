# [ib-matcher](ib-matcher/README.md)
[![crates.io](https://img.shields.io/crates/v/ib-matcher.svg)](https://crates.io/crates/ib-matcher)
[![Documentation](https://docs.rs/ib-matcher/badge.svg)](https://docs.rs/ib-matcher)
[![License](https://img.shields.io/crates/l/ib-matcher.svg)](LICENSE.txt)

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
- [glob()-style](https://docs.rs/ib-matcher/latest/ib_matcher/syntax/glob/) pattern matching (i.e. `?`, `*` and `**`)
  - Support two seperators (`//`) or a complement separator (`\`) as a glob star (`*/**`).
- [Regular expression](https://docs.rs/ib-matcher/latest/ib_matcher/regex/)
  - Support the same syntax as [`regex`](https://docs.rs/regex/), including wildcards, repetitions, alternations, groups, etc.
  - Support [custom matching callbacks](https://docs.rs/ib-matcher/latest/ib_matcher/regex/cp/struct.Regex.html#custom-matching-callbacks), which can be used to implement ad hoc look-around, backreferences, balancing groups/recursion/subroutines, combining domain-specific parsers, etc.
- Relatively high performance

And all of the above features are optional. You don't need to pay the performance and binary size cost for features you don't use.

You can also use [ib-pinyin](#ib-pinyin) if you only need Chinese pinyin match, which is simpler and more stable.

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

## [ib-pinyin](ib-pinyin/README.md)
一个高性能 Rust 拼音查询、匹配库。

- 支持以下拼音编码方案：
  - 简拼（“py”）
  - 全拼（“pinyin”）
  - 带声调全拼（“pin1yin1”）
  - Unicode（“pīnyīn”）
  - 智能 ABC 双拼
  - 拼音加加双拼
  - 微软双拼
  - 华宇双拼（紫光双拼）
  - 小鹤双拼
  - 自然码双拼
- 支持多音字。
- 支持混合匹配多种拼音编码方案，默认匹配简拼和全拼。
- 默认小写字母匹配拼音或字母，大写字母只匹配字母。
- 支持 Unicode 辅助平面汉字。

支持 C、AHK2。

### [Rust](ib-pinyin)
[![crates.io](https://img.shields.io/crates/v/ib-pinyin.svg)](https://crates.io/crates/ib-pinyin)
[![Documentation](https://docs.rs/ib-pinyin/badge.svg)](https://docs.rs/ib-pinyin)

```rust
use ib_pinyin::{matcher::PinyinMatcher, pinyin::PinyinNotation};

let matcher = PinyinMatcher::builder("pysousuoeve")
    .pinyin_notations(PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter)
    .build();
assert!(matcher.is_match("拼音搜索Everything"));
```

### [C](ib-pinyin/bindings/c)
```c
#include <ib_pinyin/ib_pinyin.h>
#include <ib_pinyin/notation.h>

// UTF-8
bool is_match = ib_pinyin_is_match_u8c(u8"pysousuoeve", u8"拼音搜索Everything", PINYIN_NOTATION_ASCII_FIRST_LETTER | PINYIN_NOTATION_ASCII);

// UTF-16
bool is_match = ib_pinyin_is_match_u16c(u"pysousuoeve", u"拼音搜索Everything", PINYIN_NOTATION_ASCII_FIRST_LETTER | PINYIN_NOTATION_ASCII);

// UTF-32
bool is_match = ib_pinyin_is_match_u32c(U"pysousuoeve", U"拼音搜索Everything", PINYIN_NOTATION_ASCII_FIRST_LETTER | PINYIN_NOTATION_ASCII);
```

### C++
[原实现](ib-pinyin-cpp)（停止维护）

### [AutoHotkey v2](ib-pinyin/bindings/ahk2)
```ahk
#Include <IbPinyin>

IsMatch := IbPinyin_Match("pysousuoeve", "拼音搜索Everything")
; 指定拼音编码
IsMatch := IbPinyin_Match("pysousuoeve", "拼音搜索Everything", IbPinyin_AsciiFirstLetter | IbPinyin_Ascii)
; 获取匹配范围
IsMatch := IbPinyin_Match("pysousuoeve", "拼音搜索Everything", IbPinyin_AsciiFirstLetter | IbPinyin_Ascii, &start, &end)

; 中文 API
是否匹配 := 拼音_匹配("pysousuoeve", "拼音搜索Everything")
; 指定拼音编码
是否匹配 := 拼音_匹配("pysousuoeve", "拼音搜索Everything", 拼音_简拼 | 拼音_全拼)
; 获取匹配范围
是否匹配 := 拼音_匹配("pysousuoeve", "拼音搜索Everything", 拼音_简拼 | 拼音_全拼, &开始位置, &结束位置)
```
[下载](https://github.com/Chaoses-Ib/ib-matcher/releases)

## [ib-romaji](ib-romaji/README.md)
[![crates.io](https://img.shields.io/crates/v/ib-romaji.svg)](https://crates.io/crates/ib-romaji)
[![Documentation](https://docs.rs/ib-romaji/badge.svg)](https://docs.rs/ib-romaji)

A fast Japanese romanizer.

## [ib-unicode](ib-unicode/README.md)
[![crates.io](https://img.shields.io/crates/v/ib-unicode.svg)](https://crates.io/crates/ib-unicode)
[![Documentation](https://docs.rs/ib-unicode/badge.svg)](https://docs.rs/ib-unicode)

Unicode utils.

## 其它拼音相关项目
语言 | 库 | 拼音 | 双拼 | 词典 | 匹配 | 其它
--- | --- | --- | --- | --- | --- | ---
Rust <br /> (C, AHK2) | ib-matcher/ib-pinyin | ✔️ Unicode | ✔️ | ❌ | ✔️ | 支持日文；支持正则表达式；性能优先；支持 Unicode 辅助平面汉字
Rust <br /> ([Node.js](https://github.com/Brooooooklyn/pinyin)) | [rust-pinyin](https://github.com/mozillazg/rust-pinyin) | ✔️ Unicode | ❌ | ❌ | ❌
Rust | [rust-pinyin](https://github.com/samlink/rust_pinyin) | 简拼 | ❌ | ❌ | ❌
C# | [ToolGood.Words.Pinyin](https://github.com/toolgood/ToolGood.Words.Pinyin) | ✔️ | ❌ | ❌ | 单编码？
C# | [TinyPinyin.Net](https://github.com/hstarorg/TinyPinyin.Net) | ✔️ | ❌ | ❌ | ❌
C# | [Romanization.NET](https://github.com/zedseven/Romanization.NET) | Unicode | ❌ | | ❌ | 支持日文、韩文、俄文、希腊文
Java | [PinIn](https://github.com/Towdium/PinIn) | ✔️ | ✔️ | ❌ | ✔️ | 支持注音输入法、模糊音
Java | [TinyPinyin](https://github.com/promeG/TinyPinyin) | ✔️ | ❌ | ✔️ | ❌
Go | [go-pinyin](https://github.com/mozillazg/go-pinyin) | ✔️ | ❌ | ✔️ | ❌
Python | [python-pinyin](https://github.com/mozillazg/python-pinyin) | ✔️ | ❌ | ✔️ | ❌
TS | [pinyin-pro](https://github.com/zh-lx/pinyin-pro) | ✔️ | ❌ | ❌ | ✔️
JS | [pinyin-match](https://github.com/xmflswood/pinyin-match) | ✔️ | ❌ | ❌ | 单编码 | 匹配时忽略空白
JS | [pinyin-engine](https://github.com/aui/pinyin-engine) | ✔️ | ❌ | ❌ | 单编码
JS | [pinyin](https://github.com/hotoo/pinyin) | ✔️ | ❌ | ✔️ | ❌
JS | [pinyinjs](https://github.com/sxei/pinyinjs) | ✔️ Unicode | ❌ | ❌ | ❌
Perl <br /> ([Rust](https://github.com/chowdhurya/rust-unidecode/), [Java](https://github.com/xuender/unidecode), [Python](https://github.com/avian2/unidecode), [Ruby](http://www.rubydoc.info/gems/unidecode/1.0.0/frames), [JS](https://www.npmjs.org/package/unidecode), [PHP](https://github.com/silverstripe-labs/silverstripe-unidecode)) | [Text::Unidecode](https://metacpan.org/pod/Text::Unidecode) | ✔️ | ❌ | ❌ | ❌ | 支持文字广泛

数据库：
- [Simple tokenizer: 支持中文和拼音的 SQLite fts5 全文搜索扩展 ｜ A SQLite3 fts5 tokenizer which supports Chinese and PinYin](https://github.com/wangfenjin/simple)

文件搜索/启动器：
- [IbEverythingExt: Everything Everything 拼音搜索、ローマ字検索、快速选择扩展](https://github.com/Chaoses-Ib/IbEverythingExt)（基于 ib-matcher）
- [Listary](https://www.listary.com/)（简拼、全拼）

文件管理：
- 资源管理器
  - [资源管理器拼音搜索扩展](https://github.com/sxzxs/explore_select_items)（基于 ib-matcher）
- [Directory Opus](https://github.com/Chaoses-Ib/DirectoryOpus)（仅简拼）
- Total Commander：[QuickSearch eXtended](https://www.ghisler.ch/board/viewtopic.php?t=22592)（仅简拼）

终端：
- [bash-pinyin-completion-rs: Simple completion script for pinyin, written in rust.](https://github.com/wxiwnd/bash-pinyin-completion-rs)（基于 ib-matcher）

文本编辑：
- Visual Studio
  - [ChinesePinyinIntelliSenseExtender: VisualStudio中文代码拼音补全拓展](https://github.com/stratosblue/ChinesePinyinIntelliSenseExtender)
  - [VSIXChineseCompletion: Visual Studio (CSharp) 中文代码补全 (使用拼音补全中文)](https://github.com/sharpoverflow/VSIXChineseCompletion)
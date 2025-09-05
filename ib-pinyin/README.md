# ib-pinyin
[![crates.io](https://img.shields.io/crates/v/ib-pinyin.svg)](https://crates.io/crates/ib-pinyin)
[![Documentation](https://docs.rs/ib-pinyin/badge.svg)](https://docs.rs/ib-pinyin)
[![License](https://img.shields.io/crates/l/ib-pinyin.svg)](../LICENSE.txt)

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

支持 [C](bindings/c/README.md)、[AHK2](bindings/ahk2/README.md)。

## Usage
```rust
use ib_pinyin::{matcher::PinyinMatcher, pinyin::PinyinNotation};

let matcher = PinyinMatcher::builder("pysousuoeve")
    .pinyin_notations(PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter)
    .build();
assert!(matcher.is_match("拼音搜索Everything"));
```

## Testing
```sh
cargo hack test --feature-powerset
```
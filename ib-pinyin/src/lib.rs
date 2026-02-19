//! 一个高性能拼音查询、匹配库。
//!
/*!
See [Pinyin](https://en.wikipedia.org/wiki/Pinyin) for what is pinyin.
*/
//! - 支持以下拼音编码方案：
//!   - 简拼（“py”）
//!   - 全拼（“pinyin”）
//!   - 带声调全拼（“pin1yin1”）
//!   - Unicode（“pīnyīn”）
//!   - 智能 ABC 双拼
//!   - 拼音加加双拼
//!   - 微软双拼
//!   - 华宇双拼（紫光双拼）
//!   - 小鹤双拼
//!   - 自然码双拼
//! - 支持多音字。
//! - 支持混合匹配多种拼音编码方案，默认匹配简拼和全拼。
//! - 默认小写字母匹配拼音或字母，大写字母只匹配字母。
//! - 支持 Unicode 辅助平面汉字。
//!
//! ## Usage
//! ```
//! use ib_pinyin::{matcher::PinyinMatcher, pinyin::PinyinNotation};
//!
//! let matcher = PinyinMatcher::builder("pysousuoeve")
//!     .pinyin_notations(PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter)
//!     .build();
//! assert!(matcher.is_match("拼音搜索Everything"));
//! ```
//!
//! ## Features
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "doc", doc = document_features::document_features!())]
pub mod matcher;
#[cfg(feature = "minimal")]
pub mod minimal;
pub mod pinyin;

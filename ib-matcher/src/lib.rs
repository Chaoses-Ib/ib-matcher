//! A multilingual and fast string matcher, supports 拼音匹配 (Chinese pinyin match) and ローマ字検索 (Japanese romaji match).
//!
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
//!
//! ## Performance
//! The following `Cargo.toml` settings are recommended if best performance is desired:
//! ```toml
//! [profile.release]
//! lto = "fat"
//! codegen-units = 1
//! ```
//! These can improve the performance by 5~10% at most.
//!
//! ## Features
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
#[cfg(feature = "syntax")]
pub mod syntax;
pub mod unicode;

#[cfg(feature = "romaji")]
pub use ib_romaji as romaji;

mod private {
    pub trait Sealed {}
}
use private::Sealed;

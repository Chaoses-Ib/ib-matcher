/**
A meta regex engine optimized for literal pattern matching.

Compared to [`cp::Regex`](crate::regex::cp::Regex), this engine has much better performance if and only if the pattern is a literal string (i.e. plain text, optionally with pinyin/romaji match). It is suitable if the pattern is often (but not always) a literal string, such as matching file names, paths, etc.

It has the following limitations though:
- UTF-8 only. The pattern and haystack must be valid UTF-8, otherwise the engine may panic.
- No `find_iter()` and `captures_iter()` at the moment.
- No `build_many()`.
- No custom matching callback support.

The primary type in this module is [`Regex`].

## Design
When the pattern is a literal string, [`cp::Regex`](crate::regex::cp::Regex) is much slower than [`IbMatcher`](crate::matcher::IbMatcher). This engine uses enum dispatch to utilize the performance of [`IbMatcher`](crate::matcher::IbMatcher) if the pattern is a literal string, and fall back to [`cp::Regex`](crate::regex::cp::Regex) for other patterns.
*/
mod regex;

pub use regex::{BuildError, Builder, Config, Regex};

/*!
glob()-style pattern matching syntax support.

Supported syntax:
- [`parse_wildcard_path`]: `?`, `*` and `**`, optionally with [`GlobExtConfig`].
*/
//! - [`GlobExtConfig`]: Two seperators (`//`) or a complement separator (`\`) as a glob star (`*/**`).
/*!

The following examples match glob syntax using [`ib_matcher::regex`](crate::regex) engines.

## Example
```
// cargo add ib-matcher --features syntax-glob,regex
use ib_matcher::{regex::lita::Regex, syntax::glob::{parse_wildcard_path, PathSeparator}};

let re = Regex::builder()
    .build_from_hir(
        parse_wildcard_path()
            .separator(PathSeparator::Windows)
            .call(r"Win*\*\*.exe"),
    )
    .unwrap();
assert!(re.is_match(r"C:\Windows\System32\notepad.exe"));

let re = Regex::builder()
    .build_from_hir(
        parse_wildcard_path()
            .separator(PathSeparator::Windows)
            .call(r"Win**.exe"),
    )
    .unwrap();
assert!(re.is_match(r"C:\Windows\System32\notepad.exe"));
```

## With `IbMatcher`
```
use ib_matcher::{
    matcher::MatchConfig,
    regex::lita::Regex,
    syntax::glob::{parse_wildcard_path, PathSeparator}
};

let re = Regex::builder()
    .ib(MatchConfig::builder().pinyin(Default::default()).build())
    .build_from_hir(
        parse_wildcard_path()
            .separator(PathSeparator::Windows)
            .call(r"win**pyss.exe"),
    )
    .unwrap();
assert!(re.is_match(r"C:\Windows\System32\拼音搜索.exe"));
```
*/
use std::{borrow::Cow, path::MAIN_SEPARATOR};

use bon::{builder, Builder};
use logos::Logos;
use regex_syntax::hir::{Class, ClassBytes, ClassBytesRange, Dot, Hir, Repetition};

/// Defaults to [`PathSeparator::Os`].
#[derive(Default, Clone, Copy)]
pub enum PathSeparator {
    /// `/` on Unix and `\` on Windows.
    #[default]
    Os,
    /// i.e. `/`
    Unix,
    /// i.e. `\`
    Windows,
    /// i.e. `/` or `\`
    Any,
}

impl PathSeparator {
    fn os_desugar() -> Self {
        if MAIN_SEPARATOR == '\\' {
            PathSeparator::Windows
        } else {
            PathSeparator::Unix
        }
    }

    fn desugar(self) -> Self {
        match self {
            PathSeparator::Os => Self::os_desugar(),
            sep => sep,
        }
    }

    pub fn is_unix_or_any(self) -> bool {
        matches!(self.desugar(), PathSeparator::Unix | PathSeparator::Any)
    }

    pub fn is_windows_or_any(self) -> bool {
        matches!(self.desugar(), PathSeparator::Windows | PathSeparator::Any)
    }

    pub fn any_byte_except(&self) -> Hir {
        match self {
            // Hir::class(Class::Bytes(ClassBytes::new([
            //     ClassBytesRange::new(0, b'\\' - 1),
            //     ClassBytesRange::new(b'\\' + 1, u8::MAX),
            // ])))
            PathSeparator::Os => Hir::dot(Dot::AnyByteExcept(MAIN_SEPARATOR as u8)),
            PathSeparator::Unix => Hir::dot(Dot::AnyByteExcept(b'/')),
            PathSeparator::Windows => Hir::dot(Dot::AnyByteExcept(b'\\')),
            PathSeparator::Any => Hir::class(Class::Bytes(ClassBytes::new([
                ClassBytesRange::new(0, b'/' - 1),
                ClassBytesRange::new(b'/' + 1, b'\\' - 1),
                ClassBytesRange::new(b'\\' + 1, u8::MAX),
            ]))),
        }
    }

    // fn with_complement_char(&self) -> Option<(char, char)> {
    //     match self {
    //         PathSeparator::Os => Self::os_desugar().with_complement_char(),
    //         PathSeparator::Unix => Some(('/', '\\')),
    //         PathSeparator::Windows => Some(('\\', '/')),
    //         PathSeparator::Any => None,
    //     }
    // }

    /// The complement path separator of the current OS, i.e. `/` on Windows and `\` on Unix.
    pub fn os_complement() -> PathSeparator {
        if MAIN_SEPARATOR == '/' {
            PathSeparator::Windows
        } else {
            PathSeparator::Unix
        }
    }
}

#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum GlobStar {
    /// i.e. `*`, only match within the current component.
    Current,
    /// i.e. `**`, match anywhere, from the current component to children.
    Any,
    /// i.e. `*/**`, match from the current component to and must to children.
    ToChild,
    /// i.e. `**/`, match from the current component to and must to the start of a child.
    ToChildStart,
}

impl GlobStar {
    pub fn to_pattern(&self, separator: PathSeparator) -> &'static str {
        match self {
            GlobStar::Current => "*",
            GlobStar::Any => "**",
            GlobStar::ToChild => {
                if separator.is_unix_or_any() {
                    "*/**"
                } else {
                    r"*\**"
                }
            }
            GlobStar::ToChildStart => {
                if separator.is_unix_or_any() {
                    "**/"
                } else {
                    r"**\"
                }
            }
        }
    }
}

/// See [`GlobExtConfig`].
#[derive(Logos, Debug, PartialEq)]
enum GlobExtToken {
    #[token("/")]
    SepUnix,

    #[token(r"\")]
    SepWin,

    #[token("//")]
    TwoSepUnix,

    #[token(r"\\")]
    TwoSepWin,

    /// Plain text.
    #[regex(r"[^/\\]+")]
    Text,
}

/// Support two seperators (`//`) or a complement separator (`\`) as a glob star (`*/**`).
///
/// Optional extensions:
/// - [`two_separator_as_star`](GlobExtConfigBuilder::two_separator_as_star): `\\` as `*\**`.
/// - [`separator_as_star`](GlobExtConfigBuilder::separator_as_star): `/` as `*\**`.
#[derive(Builder, Default, Clone, Copy)]
pub struct GlobExtConfig {
    /// - `sep`: You likely want to use [`PathSeparator::Any`].
    /// - `star`:
    ///   - [`GlobStar::ToChild`]: Replace `\\` with `*\**` for Windows and vice versa for Unix.
    ///
    /// Used by voidtools' Everything.
    #[builder(with = |sep: PathSeparator, star: GlobStar| (sep, star))]
    two_separator_as_star: Option<(PathSeparator, GlobStar)>,
    /// - `sep`: You likely want to use [`PathSeparator::os_complement()`].
    /// - `star`:
    ///   - [`GlobStar::ToChild`]: Replace `/` with `*\**` for Windows and vice versa for Unix.
    ///
    ///     e.g. `xx/hj` can match `xxzl\sj\7yhj` (`学习资料\时间\7月合集` with pinyin match) for Windows.
    ///   - [`GlobStar::ToChildStart`]: Replace `/` with `**\` for Windows and vice versa for Unix.
    ///
    ///     For example:
    ///     - `foo/alice` can, but `foo/lice` can't match `foo\bar\alice` for Windows.
    ///     - `xx/7y` can, but `xx/hj` can't match `xxzl\sj\7yhj` (`学习资料\时间\合集7月` with pinyin match) for Windows.
    ///
    /// Used by IbEverythingExt.
    #[builder(with = |sep: PathSeparator, star: GlobStar| (sep, star))]
    separator_as_star: Option<(PathSeparator, GlobStar)>,
}

impl GlobExtConfig {
    /// The config used by IbEverythingExt. Suitable for common use cases.
    pub fn new_ev() -> Self {
        GlobExtConfig {
            two_separator_as_star: Some((PathSeparator::Any, GlobStar::ToChild)),
            separator_as_star: Some((PathSeparator::os_complement(), GlobStar::ToChildStart)),
        }
    }

    #[cfg(test)]
    fn desugar_single<'p>(&self, pattern: &'p str, to_separator: PathSeparator) -> Cow<'p, str> {
        let mut pattern = Cow::Borrowed(pattern);
        if let Some((sep, star)) = self.two_separator_as_star {
            let star_pattern = star.to_pattern(to_separator);
            pattern = match sep.desugar() {
                PathSeparator::Os => unreachable!(),
                PathSeparator::Unix => pattern.replace("//", star_pattern),
                PathSeparator::Windows => pattern.replace(r"\\", star_pattern),
                PathSeparator::Any => pattern
                    .replace("//", star_pattern)
                    .replace(r"\\", star_pattern),
            }
            .into();
        }
        if let Some((sep, star)) = self.separator_as_star {
            let star_pattern = star.to_pattern(to_separator);
            pattern = match sep.desugar() {
                PathSeparator::Os => unreachable!(),
                PathSeparator::Unix => pattern.replace('/', star_pattern),
                PathSeparator::Windows => pattern.replace('\\', star_pattern),
                PathSeparator::Any => {
                    if to_separator.is_unix_or_any() {
                        pattern
                            .replace('/', star_pattern)
                            .replace('\\', star_pattern)
                    } else {
                        pattern
                            .replace('\\', star_pattern)
                            .replace('/', star_pattern)
                    }
                }
            }
            .into();
        }
        #[cfg(test)]
        dbg!(&pattern);
        pattern
    }

    /// - `to_separator`: The separator the pattern should be desugared to.
    pub fn desugar<'p>(&self, pattern: &'p str, to_separator: PathSeparator) -> Cow<'p, str> {
        if self.two_separator_as_star.is_none() && self.separator_as_star.is_none() {
            return Cow::Borrowed(pattern);
        }
        // TODO: desugar_single optimization?

        let mut lex = GlobExtToken::lexer(&pattern);
        let mut pattern = String::with_capacity(pattern.len());
        let sep_unix = self
            .separator_as_star
            .filter(|(sep, _)| sep.is_unix_or_any())
            .map(|(_, star)| star.to_pattern(to_separator))
            .unwrap_or("/");
        let sep_win = self
            .separator_as_star
            .filter(|(sep, _)| sep.is_windows_or_any())
            .map(|(_, star)| star.to_pattern(to_separator))
            .unwrap_or(r"\");
        let two_sep_unix = self
            .two_separator_as_star
            .filter(|(sep, _)| sep.is_unix_or_any())
            .map(|(_, star)| star.to_pattern(to_separator))
            .unwrap_or("//");
        let two_sep_win = self
            .two_separator_as_star
            .filter(|(sep, _)| sep.is_windows_or_any())
            .map(|(_, star)| star.to_pattern(to_separator))
            .unwrap_or(r"\\");
        while let Some(Ok(token)) = lex.next() {
            pattern.push_str(match token {
                GlobExtToken::SepUnix => sep_unix,
                GlobExtToken::SepWin => sep_win,
                GlobExtToken::TwoSepUnix => two_sep_unix,
                GlobExtToken::TwoSepWin => two_sep_win,
                GlobExtToken::Text => lex.slice(),
            });
        }
        #[cfg(test)]
        dbg!(&pattern);
        Cow::Owned(pattern)
    }
}

/// See [`parse_wildcard_path`].
#[derive(Logos, Debug, PartialEq)]
pub enum WildcardPathToken {
    /// Equivalent to `[^/]` on Unix and `[^\\]` on Windows.
    #[token("?")]
    Any,

    /// Equivalent to `[^/]*` on Unix and `[^\\]*` on Windows.
    #[token("*")]
    Star,

    /// Equivalent to `.*`.
    #[token("**")]
    GlobStar,

    /// Plain text.
    #[regex("[^*?]+")]
    Text,
}

/// Wildcard-only path glob syntax flavor, including `?`, `*` and `**`.
///
/// Used by voidtools' Everything, etc.
#[builder]
pub fn parse_wildcard_path(
    #[builder(finish_fn)] pattern: &str,
    /// The path separator used in the haystacks to be matched.
    ///
    /// Only have effect on `?` and `*`.
    separator: PathSeparator,
    #[builder(default)] ext: GlobExtConfig,
) -> Hir {
    // Desugar
    let pattern = ext.desugar(pattern, separator);

    let mut lex = WildcardPathToken::lexer(&pattern);
    let mut hirs = Vec::new();
    while let Some(Ok(token)) = lex.next() {
        hirs.push(match token {
            WildcardPathToken::Any => separator.any_byte_except(),
            WildcardPathToken::Star => Hir::repetition(Repetition {
                min: 0,
                max: None,
                greedy: true,
                sub: separator.any_byte_except().into(),
            }),
            WildcardPathToken::GlobStar => Hir::repetition(Repetition {
                min: 0,
                max: None,
                greedy: true,
                sub: Hir::dot(Dot::AnyByte).into(),
            }),
            WildcardPathToken::Text => Hir::literal(lex.slice().as_bytes()),
        });
    }
    Hir::concat(hirs)
}

#[cfg(test)]
mod tests {
    use regex_syntax::ParserBuilder;

    use crate::{matcher::MatchConfig, regex::lita::Regex};

    use super::*;

    #[test]
    fn wildcard_path_token() {
        let input = "*text?more*?text**end";
        let mut lexer = WildcardPathToken::lexer(input);
        assert_eq!(lexer.next(), Some(Ok(WildcardPathToken::Star)));
        assert_eq!(lexer.next(), Some(Ok(WildcardPathToken::Text)));
        assert_eq!(lexer.next(), Some(Ok(WildcardPathToken::Any)));
        assert_eq!(lexer.next(), Some(Ok(WildcardPathToken::Text)));
        assert_eq!(lexer.next(), Some(Ok(WildcardPathToken::Star)));
        assert_eq!(lexer.next(), Some(Ok(WildcardPathToken::Any)));
        assert_eq!(lexer.next(), Some(Ok(WildcardPathToken::Text)));
        assert_eq!(lexer.next(), Some(Ok(WildcardPathToken::GlobStar)));
        assert_eq!(lexer.next(), Some(Ok(WildcardPathToken::Text)));
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn wildcard_path() {
        let hir1 = ParserBuilder::new()
            .utf8(false)
            .build()
            .parse(r"(?s-u)[^\\]a[^\\]*b.*c")
            .unwrap();
        println!("{:?}", hir1);

        let hir2 = parse_wildcard_path()
            .separator(PathSeparator::Windows)
            .call("?a*b**c");
        println!("{:?}", hir1);

        assert_eq!(hir1, hir2);

        let re = Regex::builder().build_from_hir(hir2).unwrap();
        assert!(re.is_match(r"1a2b33c"));
        assert!(re.is_match(r"1a\b33c") == false);

        let re = Regex::builder()
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"Win*\*\*.exe"),
            )
            .unwrap();
        assert!(re.is_match(r"C:\Windows\System32\notepad.exe"));

        let re = Regex::builder()
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"Win**.exe"),
            )
            .unwrap();
        assert!(re.is_match(r"C:\Windows\System32\notepad.exe"));

        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"win**pyss.exe"),
            )
            .unwrap();
        assert!(re.is_match(r"C:\Windows\System32\拼音搜索.exe"));
    }

    #[test]
    fn complement_separator_as_glob_star() {
        let ext = GlobExtConfig::builder()
            .separator_as_star(PathSeparator::Any, GlobStar::ToChild)
            .build();

        assert_eq!(
            ext.desugar_single(r"xx/hj", PathSeparator::Windows),
            r"xx*\**hj"
        );
        assert_eq!(ext.desugar(r"xx/hj", PathSeparator::Windows), r"xx*\**hj");
        let re = Regex::builder()
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .ext(ext)
                    .call(r"xx/hj"),
            )
            .unwrap();
        assert!(re.is_match(r"xxzl\sj\8yhj"));

        let re = Regex::builder()
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Unix)
                    .ext(ext)
                    .call(r"xx\hj"),
            )
            .unwrap();
        assert!(re.is_match(r"xxzl/sj/8yhj"));

        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .ext(ext)
                    .call(r"xx/hj"),
            )
            .unwrap();
        assert!(re.is_match(r"学习资料\时间\7月合集"));
    }
}

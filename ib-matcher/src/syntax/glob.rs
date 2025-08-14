/*!
glob()-style pattern matching syntax support.

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
    regex::cp::Regex,
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
use std::path::MAIN_SEPARATOR;

use bon::builder;
use logos::Logos;
use regex_syntax::hir::{Class, ClassBytes, ClassBytesRange, Dot, Hir, Repetition};

/// Defaults to [`PathSeparator::Os`].
#[derive(Default)]
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

    pub fn with_complement_char(&self) -> Option<(char, char)> {
        match self {
            PathSeparator::Os => Self::os_desugar().with_complement_char(),
            PathSeparator::Unix => Some(('/', '\\')),
            PathSeparator::Windows => Some(('\\', '/')),
            PathSeparator::Any => None,
        }
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
///
/// Optional extensions:
/// - [`complement_separator_as_glob_star`](ParseWildcardPathBuilder::complement_separator_as_glob_star)
#[builder]
pub fn parse_wildcard_path(
    #[builder(finish_fn)] pattern: &str,
    separator: PathSeparator,
    /// Replace `/` with `*\**` on Windows and vice versa on Unix.
    ///
    /// e.g. `xx/hj` can match `xxzl\sj\7yhj` (`学习资料\时间\7月合集` with pinyin match) on Windows.
    #[builder(default)]
    complement_separator_as_glob_star: bool,
) -> Hir {
    // Desugar
    let buf;
    let pattern = if let Some((separator, complement)) = complement_separator_as_glob_star
        .then(|| separator.with_complement_char())
        .flatten()
    {
        buf = pattern.replace(complement, &format!("*{}**", separator));
        buf.as_str()
    } else {
        pattern
    };

    let mut lex = WildcardPathToken::lexer(pattern);
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

    use crate::{matcher::MatchConfig, regex::cp::Regex};

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
        let re = Regex::builder()
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .complement_separator_as_glob_star(true)
                    .call(r"xx/hj"),
            )
            .unwrap();
        assert!(re.is_match(r"xxzl\sj\8yhj"));

        let re = Regex::builder()
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Unix)
                    .complement_separator_as_glob_star(true)
                    .call(r"xx\hj"),
            )
            .unwrap();
        assert!(re.is_match(r"xxzl/sj/8yhj"));

        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .complement_separator_as_glob_star(true)
                    .call(r"xx/hj"),
            )
            .unwrap();
        assert!(re.is_match(r"学习资料\时间\7月合集"));
    }
}

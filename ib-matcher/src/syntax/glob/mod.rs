/*!
glob()-style pattern matching syntax support.

Supported syntax:
- [`parse_wildcard_path`]: `?`, `*` and `**`, optionally with [`GlobExtConfig`].
  - Windows file name safe.

  Used by voidtools' Everything, etc.

- [`parse_glob_path`]: `?`, `*`, `[]` and `**`, optionally with [`GlobExtConfig`].
  - Parsing of `[]` is [fallible](#error-behavior).
  - Not Windows file name safe: `[]` may disturb the matching of literal `[]` in file names.

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

## Anchor modes
There are four possible anchor modes:
- Matching from the start of the string. Used by terminal auto completion.
- Matching from anywhere in the string. Used by this module.
- Matching to the end of the string. Rarely used besides matching file extensions.
- Matching the whole string (from the start to the end). Used by [voidtools' Everything](https://github.com/Chaoses-Ib/IbEverythingExt/issues/98).

This module will match from anywhere in the string by default. For other modes:
- To match from the start of the string only, you can append a `*` to the pattern (like `foo*`), which will then be consider as an anchor (by [`surrounding_wildcard_as_anchor`](ParseWildcardPathBuilder::surrounding_wildcard_as_anchor)).
- To match the whole string only, you can combine the above one with checking the returned match length at the moment.
- If you want to match to the end of the string, prepend a `*`, like `*.mp4`.

### Surrounding wildcards as anchors
> TL;DR: When not matching the whole string, enabling [`surrounding_wildcard_as_anchor`](ParseWildcardPathBuilder::surrounding_wildcard_as_anchor) let patterns like `*.mp4` matches `v.mp4` but not `v.mp4_0.webp` (it matches both if disabled). And it's enabled by default.

Besides matching the whole string, other anchor modes can have some duplicate patterns. For example, when matching from anywhere, `*.mp4` will match the same strings matched by `.mp4`; when matching from the start, `foo*` is the same as `foo`.

These duplicate patterns have no syntax error, but matching them literally probably isn't what the user want. For example, `*.mp4` actually means the match must be to the end, `foo*` actually means the match must be from the start, otherwise the user would just type `.mp4` or `foo`. And the formers also cause worse match highlight (hightlighting the whole string isn't useful).

To fix these problems, one way is to only match the whole string, another way is to treat leading and trailing wildcards differently. The user-side difference of them is how patterns like `a*b` are treated: the former requires `^a.*b$`, the latter allows `^.*a.*b.*$` (`*a*b*` in the former). The latter is more user-friendly (in my option) and can be converted to the former by adding anchor modes, so it's implemented here: [`surrounding_wildcard_as_anchor`](ParseWildcardPathBuilder::surrounding_wildcard_as_anchor), enabled by default.

Related issue: [IbEverythingExt #98](https://github.com/Chaoses-Ib/IbEverythingExt/issues/98)

### Anchors in file paths
> TL;DR: If you are matching file paths, you probably want to set `Regex::builder().thompson(PathSeparator::Windows.look_matcher_config())`.

Another problem about anchored matching is, when matching file paths, should the anchors match the start/end of the whole path or the path components (i.e. match separators)?

The default behavior is the former, for example:
```
use ib_matcher::{
    matcher::MatchConfig,
    regex::lita::Regex,
    syntax::glob::{parse_wildcard_path, PathSeparator}
};

let re = Regex::builder()
    .ib(MatchConfig::default())
    .build_from_hir(
        parse_wildcard_path()
            .separator(PathSeparator::Windows)
            .call(r"?\foo*\"),
    )
    .unwrap();
assert!(re.is_match(r"C\foobar\⑨"));
assert!(re.is_match(r"D\C\foobar\9") == false); // Doesn't match
assert!(re.is_match(r"DC\foobar\9") == false);
assert!(re.is_match(r"C\DC\foobar\9") == false);
```

If you want the latter behavior, i.e. special anchors that match `/` or `\` too, you need to set `look_matcher` in [`crate::regex::nfa::thompson::Config`], for example:
```
use ib_matcher::{
    matcher::MatchConfig,
    regex::lita::Regex,
    syntax::glob::{parse_wildcard_path, PathSeparator}
};

let re = Regex::builder()
    .ib(MatchConfig::default())
    .thompson(PathSeparator::Windows.look_matcher_config())
    .build_from_hir(
        parse_wildcard_path()
            .separator(PathSeparator::Windows)
            .call(r"?\foo*\"),
    )
    .unwrap();
assert!(re.is_match(r"C\foobar\⑨"));
assert!(re.is_match(r"D\C\foobar\9")); // Now matches
assert!(re.is_match(r"DC\foobar\9") == false);
assert!(re.is_match(r"C\DC\foobar\9") == false);
```

The latter behavior is used by voidtools' Everything.

Related issue: [IbEverythingExt #99](https://github.com/Chaoses-Ib/IbEverythingExt/issues/99)

## Character classes
<!-- Support the same syntax as in [`regex`](crate::syntax::regex#character-classes), with `^` replaced by `!`. -->

Support patterns like `[abc]`, `[a-z]`, `[!a-z]` and `[[:ascii:]]`.

Character classes can be used to escape the metacharacter: `[?]`, `[*]`, `[[]`, `[]]` match the literal characters `?`, `*`, `[`, `]` respectively.

### Error behavior
Parsing of `[]` is fallible: patterns like `a[b` are invalid.

At the moment related characters will be treated as literal characters if parsing fails.

### Examples
```
# use ib_matcher::{syntax::glob::{parse_glob_path, PathSeparator}, regex::cp::Regex};
# let is_match = |p, h| {
#     Regex::builder()
#         .build_from_hir(parse_glob_path().separator(PathSeparator::Windows).call(p))
#         .unwrap()
#         .is_match(h)
# };
// Set
assert!(is_match("a[b]z", "abz"));
assert!(is_match("a[b]z", "aBz") == false);
assert!(is_match("a[bcd]z", "acz"));

// Range
assert!(is_match("a[b-z]z", "ayz"));

// Negative set
assert!(is_match("a[!b]z", "abz") == false);
assert!(is_match("a[!b]z", "acz"));

// ASCII character class
assert!(is_match("a[[:space:]]z", "a z"));

// Escape
assert!(is_match("a[?]z", "a?z"));
assert!(is_match("a[*]z", "a*z"));
assert!(is_match("a[[]z", "a[z"));
assert!(is_match("a[-]z", "a-z"));
assert!(is_match("a[]]z", "a]z"));
assert!(is_match(r"a[\d]z", r"a\z"));

// Invalid patterns
assert!(is_match("a[b", "a[bz"));
assert!(is_match("a[[b]z", "a[[b]z"));
assert!(is_match("a[!]z", "a[!]z"));
```
*/
use std::{borrow::Cow, path::MAIN_SEPARATOR};

use bon::{builder, Builder};
use logos::Logos;
use regex_automata::{nfa::thompson, util::look::LookMatcher};
use regex_syntax::{
    hir::{
        Class, ClassBytes, ClassBytesRange, ClassUnicode, ClassUnicodeRange, Dot, Hir, Repetition,
    },
    ParserBuilder,
};

use util::SurroundingWildcardHandler;

mod util;

#[derive(Logos, Clone, Copy, Debug, PartialEq)]
pub enum WildcardToken {
    /// Equivalent to `.`.
    #[token("?")]
    Any,

    /// Equivalent to `.*`.
    #[token("*")]
    Star,

    /// Plain text.
    #[regex("[^*?]+")]
    Text,
}

/// Defaults to [`PathSeparator::Os`], i.e. `/` on Unix and `\` on Windows.
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

    fn literal(&self) -> Hir {
        match self.desugar() {
            PathSeparator::Os => unreachable!(),
            PathSeparator::Unix => Hir::literal(*b"/"),
            PathSeparator::Windows => Hir::literal(*b"\\"),
            PathSeparator::Any => Hir::class(Class::Bytes(ClassBytes::new([
                ClassBytesRange::new(b'/', b'/'),
                ClassBytesRange::new(b'\\', b'\\'),
            ]))),
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

    pub fn any_char_except(&self) -> Hir {
        match self {
            PathSeparator::Os => Hir::dot(Dot::AnyCharExcept(MAIN_SEPARATOR)),
            PathSeparator::Unix => Hir::dot(Dot::AnyCharExcept('/')),
            PathSeparator::Windows => Hir::dot(Dot::AnyCharExcept('\\')),
            PathSeparator::Any => Hir::class(Class::Unicode(ClassUnicode::new([
                ClassUnicodeRange::new('\0', '.'),
                ClassUnicodeRange::new('0', '['),
                ClassUnicodeRange::new(']', char::MAX),
            ]))),
        }
    }

    /// Does not support `PathSeparator::Any` yet.
    pub fn look_matcher(&self) -> LookMatcher {
        debug_assert!(!matches!(self, PathSeparator::Any));

        let mut lookm = LookMatcher::new();
        lookm.set_line_terminator(if self.is_unix_or_any() { b'/' } else { b'\\' });
        lookm
    }

    /// Does not support `PathSeparator::Any` yet.
    pub fn look_matcher_config(&self) -> thompson::Config {
        thompson::Config::new().look_matcher(self.look_matcher())
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
#[derive(Logos, Clone, Copy, Debug, PartialEq)]
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

    #[token("/")]
    SepUnix,

    #[token(r"\")]
    SepWin,

    /// Plain text.
    #[regex(r"[^*?/\\]+")]
    Text,
}

/// Wildcard-only path glob syntax flavor, including `?`, `*` and `**`.
///
/// Used by voidtools' Everything, etc.
#[builder]
pub fn parse_wildcard_path(
    #[builder(finish_fn)] pattern: &str,
    /// The separator used in the pattern. Can be different from the one used in the haystacks to be matched.
    ///
    /// Defaults to the same as `separator`. You may want to use [`PathSeparator::Any`] instead.
    pattern_separator: Option<PathSeparator>,
    /// The path separator used in the haystacks to be matched.
    ///
    /// Only have effect on `?` and `*`.
    separator: PathSeparator,
    /// See [`surrounding wildcards as anchors`](super::glob#surrounding-wildcards-as-anchors).
    #[builder(default = true)]
    surrounding_wildcard_as_anchor: bool,
    #[builder(default)] ext: GlobExtConfig,
) -> Hir {
    let pattern_separator = pattern_separator.unwrap_or(separator);

    // Desugar
    let pattern = ext.desugar(pattern, separator);

    let mut lex = WildcardPathToken::lexer(&pattern);
    let mut hirs = Vec::new();
    let mut surrounding_handler =
        surrounding_wildcard_as_anchor.then(|| SurroundingWildcardHandler::new(pattern_separator));
    while let Some(Ok(token)) = lex.next() {
        if let Some(h) = &mut surrounding_handler {
            if h.skip(token, &mut hirs, &lex) {
                continue;
            }
        }

        hirs.push(match token {
            WildcardPathToken::Any => separator.any_char_except(),
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
            WildcardPathToken::SepUnix if pattern_separator.is_unix_or_any() => separator.literal(),
            WildcardPathToken::SepWin if pattern_separator.is_windows_or_any() => {
                separator.literal()
            }
            WildcardPathToken::Text | WildcardPathToken::SepUnix | WildcardPathToken::SepWin => {
                Hir::literal(lex.slice().as_bytes())
            }
        });
    }

    if let Some(h) = surrounding_handler {
        h.insert_anchors(&mut hirs);
    }

    Hir::concat(hirs)
}

/// See [`parse_glob_path`].
#[derive(Logos, Clone, Copy, Debug, PartialEq)]
pub enum GlobPathToken {
    /// Equivalent to `[^/]` on Unix and `[^\\]` on Windows.
    #[token("?")]
    Any,

    /// Equivalent to `[^/]*` on Unix and `[^\\]*` on Windows.
    #[token("*")]
    Star,

    /// `[...]`.
    #[regex(r"\[[^\]]+\]\]?")]
    Class,

    /// Equivalent to `.*`.
    #[token("**")]
    GlobStar,

    #[token("/")]
    SepUnix,

    #[token(r"\")]
    SepWin,

    /// Plain text.
    #[regex(r"[^*?\[\]/\\]+")]
    Text,
}

/// glob path syntax flavor, including `?`, `*`, `[]` and `**`.
#[builder]
pub fn parse_glob_path(
    #[builder(finish_fn)] pattern: &str,
    /// The separator used in the pattern. Can be different from the one used in the haystacks to be matched.
    ///
    /// Defaults to the same as `separator`. You may want to use [`PathSeparator::Any`] instead.
    pattern_separator: Option<PathSeparator>,
    /// The path separator used in the haystacks to be matched.
    ///
    /// Only have effect on `?` and `*`.
    separator: PathSeparator,
    /// See [`surrounding wildcards as anchors`](super::glob#surrounding-wildcards-as-anchors).
    #[builder(default = true)]
    surrounding_wildcard_as_anchor: bool,
    #[builder(default)] ext: GlobExtConfig,
) -> Hir {
    let pattern_separator = pattern_separator.unwrap_or(separator);

    // Desugar
    let pattern = ext.desugar(pattern, separator);

    let mut lex = GlobPathToken::lexer(&pattern);
    let mut hirs = Vec::new();
    let mut surrounding_handler =
        surrounding_wildcard_as_anchor.then(|| SurroundingWildcardHandler::new(pattern_separator));
    let mut parser = ParserBuilder::new().unicode(false).utf8(false).build();
    while let Some(Ok(token)) = lex.next() {
        if let Some(h) = &mut surrounding_handler {
            if h.skip(token, &mut hirs, &lex) {
                continue;
            }
        }

        hirs.push(match token {
            GlobPathToken::Any => separator.any_char_except(),
            GlobPathToken::Star => Hir::repetition(Repetition {
                min: 0,
                max: None,
                greedy: true,
                sub: separator.any_byte_except().into(),
            }),
            GlobPathToken::GlobStar => Hir::repetition(Repetition {
                min: 0,
                max: None,
                greedy: true,
                sub: Hir::dot(Dot::AnyByte).into(),
            }),
            GlobPathToken::Class => {
                let s = lex.slice();
                match s {
                    "[[]" => Hir::literal("[".as_bytes()),
                    // "[!]" => Hir::literal("!".as_bytes()),
                    _ => {
                        // Life is short
                        match parser.parse(&s.replace("[!", "[^").replace(r"\", r"\\")) {
                            Ok(hir) => hir,
                            Err(_e) => {
                                #[cfg(test)]
                                println!("{_e}");
                                Hir::literal(s.as_bytes())
                            }
                        }
                    }
                }
            }
            GlobPathToken::SepUnix if pattern_separator.is_unix_or_any() => separator.literal(),
            GlobPathToken::SepWin if pattern_separator.is_windows_or_any() => separator.literal(),
            GlobPathToken::Text | GlobPathToken::SepUnix | GlobPathToken::SepWin => {
                Hir::literal(lex.slice().as_bytes())
            }
        });
    }

    if let Some(h) = surrounding_handler {
        h.insert_anchors(&mut hirs);
    }

    Hir::concat(hirs)
}

#[cfg(test)]
mod tests {
    use regex_automata::{nfa::thompson, Match};
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
            .parse(r"[^\\](?s-u)a[^\\]*b.*c")
            .unwrap();
        println!("{:?}", hir1);

        let hir2 = parse_wildcard_path()
            .separator(PathSeparator::Windows)
            .surrounding_wildcard_as_anchor(false)
            .call("?a*b**c");
        println!("{:?}", hir2);

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
    fn glob_path() {
        let is_match = |p, h| {
            Regex::builder()
                .build_from_hir(parse_glob_path().separator(PathSeparator::Windows).call(p))
                .unwrap()
                .is_match(h)
        };

        // Set
        assert!(is_match("a[b]z", "abz"));
        assert!(is_match("a[b]z", "aBz") == false);
        assert!(is_match("a[bcd]z", "acz"));

        // Range
        assert!(is_match("a[b-z]z", "ayz"));

        // Negative set
        assert!(is_match("a[!b]z", "abz") == false);
        assert!(is_match("a[!b]z", "acz"));

        // ASCII character class
        assert!(is_match("a[[:space:]]z", "a z"));

        // Escape
        assert!(is_match("a[?]z", "a?z"));
        assert!(is_match("a[*]z", "a*z"));
        assert!(is_match("a[[]z", "a[z"));
        assert!(is_match("a[-]z", "a-z"));
        assert!(is_match("a[]]z", "a]z"));
        assert!(is_match(r"a[\d]z", r"a\z"));

        // Invalid patterns
        assert!(is_match("a[b", "a[bz"));
        assert!(is_match("a[[b]z", "a[[b]z"));
        assert!(is_match("a[!]z", "a[!]z"));
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

        // Trailing sep
        let ext = GlobExtConfig::builder()
            .separator_as_star(PathSeparator::Any, GlobStar::ToChildStart)
            .build();
        let re = Regex::builder()
            .ib(MatchConfig::default())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .ext(ext)
                    .call(r"xx/"),
            )
            .unwrap();
        assert!(re.is_match(r"C:\Xxzl\sj\8yhj"));
        assert!(re.is_match(r"C:\学习\Xxzl\sj\8yhj"));
    }

    #[test]
    fn surrounding_wildcard_as_anchor() {
        // Leading *
        let re = Regex::builder()
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"*.mp4"),
            )
            .unwrap();
        assert!(re.is_match(r"瑠璃の宝石.mp4"));
        assert!(re.is_match(r"瑠璃の宝石.mp4_001947.296.webp") == false);

        // Trailing *
        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"ll*"),
            )
            .unwrap();
        assert!(re.is_match(r"瑠璃の宝石.mp4"));
        assert_eq!(re.find(r"瑠璃の宝石.mp4"), Some(Match::must(0, 0..6)));
        assert!(re.is_match(r"ruri 瑠璃の宝石.mp4") == false);

        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"ll***"),
            )
            .unwrap();
        assert_eq!(re.find(r"瑠璃の宝石.mp4"), Some(Match::must(0, 0..6)));
        assert!(re.is_match(r"ruri 瑠璃の宝石.mp4") == false);

        // Middle *
        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"ll*.mp4"),
            )
            .unwrap();
        assert!(re.is_match(r"瑠璃の宝石.mp4"));
        assert!(re.is_match(r"ruri 瑠璃の宝石.mp4"));
        assert!(re.is_match(r"ruri 瑠璃の宝石.mp4_001133.937.webp"));

        // Leading ?
        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"??.mp4"),
            )
            .unwrap();
        assert_eq!(re.find(r"宝石.mp4"), Some(Match::must(0, 0..10)));
        assert_eq!(re.find(r"瑠璃の宝石.mp4"), None);

        // Trailing ?
        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"ll???"),
            )
            .unwrap();
        assert_eq!(re.find(r"瑠璃の宝石"), Some(Match::must(0, 0..15)));
        assert!(re.is_match(r"ruri 瑠璃の宝石") == false);
    }

    #[test]
    fn surrounding_wildcard_as_anchor_path() {
        // Leading ?
        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .thompson(PathSeparator::Windows.look_matcher_config())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"?:\$RECYCLE*\"),
            )
            .unwrap();
        assert!(re.is_match(r"C:\$RECYCLE.BIN\⑨"));
        assert!(re.is_match(r"C:\$RECYCLE.BIN\9"));
        assert!(re.is_match(r"C:\$RECYCLE.BIN\99"));
        assert!(re.is_match(r"D:\C:\$RECYCLE.BIN\9"));
        assert!(re.is_match(r"DC:\$RECYCLE.BIN\9") == false);
        assert!(re.is_match(r"D:\DC:\$RECYCLE.BIN\9") == false);

        // Trailing ?
        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(Default::default()).build())
            .thompson(PathSeparator::Windows.look_matcher_config())
            .build_from_hir(
                parse_wildcard_path()
                    .separator(PathSeparator::Windows)
                    .call(r"?:\$RECYCLE*\?"),
            )
            .unwrap();
        assert!(re.is_match(r"C:\$RECYCLE.BIN\⑨"));
        assert!(re.is_match(r"C:\$RECYCLE.BIN\9"));
        assert!(re.is_match(r"C:\$RECYCLE.BIN\99") == false);
        assert!(re.is_match(r"D:\C:\$RECYCLE.BIN\9"));
        assert!(re.is_match(r"D:\C:\$RECYCLE.BIN\99") == false);
        assert!(re.is_match(r"DC:\$RECYCLE.BIN\9") == false);
        assert!(re.is_match(r"D:\DC:\$RECYCLE.BIN\9") == false);
    }
}

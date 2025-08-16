use std::sync::Arc;

use bon::bon;
use regex_automata::{
    dfa::{self, dense},
    util::{captures::GroupInfo, primitives::NonMaxUsize},
    PatternID,
};
use regex_syntax::hir::{Hir, HirKind};

use crate::{
    matcher::{
        self, config::IbMatcherWithConfig, pattern::Pattern, MatchConfig,
    },
    regex::{
        cp,
        nfa::{backtrack, thompson},
        util::{self, captures::Captures},
        Input, Match, MatchError,
    },
};

pub use crate::regex::nfa::{backtrack::Config, thompson::BuildError};

/// A compiled regular expression for searching Unicode haystacks.
///
/// A `Regex` can be used to search haystacks, split haystacks into substrings
/// or replace substrings in a haystack with a different substring. All
/// searching is done with an implicit `(?s:.)*?` at the beginning and end of
/// an pattern. To force an expression to match the whole string (or a prefix
/// or a suffix), you can use anchored search or an anchor like `^` or `$` (or `\A` and `\z`).
/**
# Overview

The most important methods are as follows:

* [`Regex::new`] compiles a regex using the default configuration. A
[`Builder`] permits setting a non-default configuration. (For example,
case insensitive matching, verbose mode and others.)
* [`Regex::is_match`] reports whether a match exists in a particular haystack.
* [`Regex::find`] reports the byte offsets of a match in a haystack, if one
exists. [`Regex::find_iter`] returns an iterator over all such matches.
* [`Regex::captures`] returns a [`Captures`], which reports both the byte
offsets of a match in a haystack and the byte offsets of each matching capture
group from the regex in the haystack.
[`Regex::captures_iter`] returns an iterator over all such matches.
*/
/// # Example
///
/// ```
/// use ib_matcher::regex::lita::Regex;
///
/// let re = Regex::new(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$")?;
/// assert!(re.is_match("2010-03-14"));
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
/**
With `IbMatcher`'s Chinese pinyin and Japanese romaji matching:
```
// cargo add ib-matcher --features regex,pinyin,romaji
use ib_matcher::{
    matcher::{MatchConfig, PinyinMatchConfig, RomajiMatchConfig},
    regex::{lita::Regex, Match},
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
*/
/// For more examples and the syntax, see [`crate::regex`].
///
/// # Case insensitivity
/// To enable case insensitivity:
/// ```
/// use ib_matcher::{matcher::{PinyinMatchConfig, PlainMatchConfig, MatchConfig}, regex::lita::Regex};
///
/// let re = Regex::builder().ib(MatchConfig::default()).build("foo").unwrap();
/// assert!(re.is_match("FOO"));
///
/// // Alternatively, with `case_insensitive()`:
/// let re = Regex::builder()
///     .ib(MatchConfig::builder()
///         .case_insensitive(true)
///         .pinyin(PinyinMatchConfig::default())
///         .build())
///     .build("pyss")
///     .unwrap();
/// assert!(re.is_match("PY搜索"));
/// ```
/// Note that enabling `syntax.case_insensitive` will make `ib` (i.e. pinyin and romaji match) doesn't work at the moment. You should only set [`MatchConfigBuilder::case_insensitive`](crate::matcher::MatchConfigBuilder::case_insensitive) ([`PlainMatchConfig::case_insensitive`](crate::matcher::PlainMatchConfig::case_insensitive)).
///
/// If you need case insensitive character classes, you need to write `(?i:[a-z])` instead at the moment.
///
/// # Synchronization and cloning
///
/// In order to make the `Regex` API convenient, most of the routines hide
/// the fact that a `Cache` is needed at all. To achieve this, a [memory
/// pool](automata::util::pool::Pool) is used internally to retrieve `Cache`
/// values in a thread safe way that also permits reuse. This in turn implies
/// that every such search call requires some form of synchronization. Usually
/// this synchronization is fast enough to not notice, but in some cases, it
/// can be a bottleneck. This typically occurs when all of the following are
/// true:
///
/// * The same `Regex` is shared across multiple threads simultaneously,
/// usually via a [`util::lazy::Lazy`](automata::util::lazy::Lazy) or something
/// similar from the `once_cell` or `lazy_static` crates.
/// * The primary unit of work in each thread is a regex search.
/// * Searches are run on very short haystacks.
///
/// This particular case can lead to high contention on the pool used by a
/// `Regex` internally, which can in turn increase latency to a noticeable
/// effect. This cost can be mitigated in one of the following ways:
///
/// * Use a distinct copy of a `Regex` in each thread, usually by cloning it.
/// Cloning a `Regex` _does not_ do a deep copy of its read-only component.
/// But it does lead to each `Regex` having its own memory pool, which in
/// turn eliminates the problem of contention. In general, this technique should
/// not result in any additional memory usage when compared to sharing the same
/// `Regex` across multiple threads simultaneously.
/// * Use lower level APIs, like [`Regex::try_find`], which permit passing
/// a `Cache` explicitly. In this case, it is up to you to determine how best
/// to provide a `Cache`. For example, you might put a `Cache` in thread-local
/// storage if your use case allows for it.
///
/// Overall, this is an issue that happens rarely in practice, but it can
/// happen.
///
/// # Warning: spin-locks may be used in alloc-only mode
///
/// When this crate is built without the `std` feature and the high level APIs
/// on a `Regex` are used, then a spin-lock will be used to synchronize access
/// to an internal pool of `Cache` values. This may be undesirable because
/// a spin-lock is [effectively impossible to implement correctly in user
/// space][spinlocks-are-bad]. That is, more concretely, the spin-lock could
/// result in a deadlock.
///
/// [spinlocks-are-bad]: https://matklad.github.io/2020/01/02/spinlocks-considered-harmful.html
///
/// If one wants to avoid the use of spin-locks when the `std` feature is
/// disabled, then you must use APIs that accept a `Cache` value explicitly.
/// For example, [`Regex::try_find`].
#[derive(Clone)]
pub struct Regex<'a> {
    /// The actual regex implementation.
    imp: RegexI<'a>,
}

#[derive(Clone)]
enum RegexI<'a> {
    Ib(Arc<IbMatcherWithConfig<'a>>),
    Cp { dfa: dfa::regex::Regex, cp: cp::Regex<'a> },
}

#[bon]
impl<'a> Regex<'a> {
    pub fn new(pattern: &str) -> Result<Self, BuildError> {
        Self::builder().build(pattern)
    }

    pub fn config() -> thompson::Config {
        thompson::Config::new()
    }

    /// Return a builder for configuring the construction of a `Regex`.
    ///
    /// This is a convenience routine to avoid needing to import the
    /// [`Builder`] type in common cases.
    ///
    /// # Example: change the line terminator
    ///
    /// This example shows how to enable multi-line mode by default and change
    /// the line terminator to the NUL byte:
    ///
    /// ```
    /// use ib_matcher::regex::{lita::Regex, util::{syntax, look::LookMatcher}, Match};
    ///
    /// let mut lookm = LookMatcher::new();
    /// lookm.set_line_terminator(b'\x00');
    /// let re = Regex::builder()
    ///     .syntax(syntax::Config::new().multi_line(true))
    ///     .thompson(Regex::config().look_matcher(lookm))
    ///     .build(r"^foo$")?;
    /// let hay = "\x00foo\x00";
    /// assert_eq!(Some(Match::must(0, 1..4)), re.find(hay));
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[builder(builder_type = Builder, finish_fn(name = build_from_hir, doc {
    /// Builds a `Regex` directly from an `Hir` expression.
    ///
    /// This is useful if you needed to parse a pattern string into an `Hir`
    /// for other reasons (such as analysis or transformations). This routine
    /// permits building a `Regex` directly from the `Hir` expression instead
    /// of first converting the `Hir` back to a pattern string.
    ///
    /// When using this method, any options set via [`Builder::syntax`] are
    /// ignored. Namely, the syntax options only apply when parsing a pattern
    /// string, which isn't relevant here.
    ///
    /// If there was a problem building the underlying regex matcher for the
    /// given `Hir`, then an error is returned.
    ///
    /// # Example
    ///
    /// This example shows how one can hand-construct an `Hir` expression and
    /// build a regex from it without doing any parsing at all.
    ///
    /// ```
    /// use ib_matcher::regex::{
    ///     lita::Regex, Match,
    ///     syntax::hir::{Hir, Look},
    /// };
    ///
    /// // (?Rm)^foo$
    /// let hir = Hir::concat(vec![
    ///     Hir::look(Look::StartCRLF),
    ///     Hir::literal("foo".as_bytes()),
    ///     Hir::look(Look::EndCRLF),
    /// ]);
    /// let re = Regex::builder()
    ///     .build_from_hir(hir)?;
    /// let hay = "\r\nfoo\r\n";
    /// assert_eq!(Some(Match::must(0, 2..5)), re.find(hay));
    ///
    /// Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    }))]
    pub fn builder(
        #[builder(field)] syntax: util::syntax::Config,
        #[builder(finish_fn)] hir: Hir,
        /// If the provided `hir` is Unicode-aware, providing a ASCII-aware-only `Hir` as `hir_ascii` can improve performance.
        hir_ascii: Option<Hir>,
        #[builder(default)] dfa_dense: dfa::dense::Config,
        /// Thompson NFA config. Named `configure` to be compatible with [`regex_automata::meta::Builder`]. Although some fields are not supported and `utf8_empty` is named as `utf8` instead.
        #[builder(default)]
        thompson: thompson::Config,
        /// [`IbMatcher`] config.
        #[builder(default = MatchConfig::builder().case_insensitive(false).build())]
        ib: MatchConfig<'a>,
        /// `IbMatcher` pattern parser.
        ///
        /// ### Example
        /// ```
        /// use ib_matcher::{regex::lita::Regex, matcher::{MatchConfig, pattern::Pattern}};
        ///
        /// let re = Regex::builder()
        ///     .ib(MatchConfig::builder().pinyin(Default::default()).build())
        ///     .ib_parser(&mut |pattern| Pattern::parse_ev(pattern).call())
        ///     .build("pinyin;py")
        ///     .unwrap();
        /// assert!(re.is_match("拼音搜索"));
        /// assert!(re.is_match("pinyin") == false);
        /// ```
        /// See [`crate::syntax::ev`] for more details.
        mut ib_parser: Option<&mut dyn FnMut(&str) -> Pattern<str>>,
        #[builder(default = backtrack::Config::new().visited_capacity(usize::MAX / 8))]
        backtrack: backtrack::Config,
    ) -> Result<Self, BuildError> {
        _ = syntax;
        #[cfg(test)]
        dbg!(&hir);

        let imp = match hir.kind() {
            HirKind::Literal(literal) => {
                let pattern = str::from_utf8(&literal.0).unwrap();
                let pattern = if let Some(ib_parser) = ib_parser.as_mut() {
                    ib_parser(pattern)
                } else {
                    pattern.into()
                };
                RegexI::Ib(IbMatcherWithConfig::with_config(pattern, ib))
            }
            _ => {
                let dfa = {
                    // We can always forcefully disable captures because DFAs do not
                    // support them.
                    let thompson = thompson
                        .clone()
                        .which_captures(thompson::WhichCaptures::None);

                    let mut compiler = thompson::Compiler::new();
                    let hir = hir_ascii.as_ref().unwrap_or(&hir);

                    let forward_nfa = compiler
                        .configure(thompson.clone())
                        .build_from_hir(hir)?;
                    // TODO: prefilter
                    // TODO: minimize?
                    // TODO: quit vs is_ascii?
                    let forward = dense::Builder::new()
                        .configure(dfa_dense.clone())
                        .build_from_nfa(&forward_nfa)
                        .unwrap();

                    let reverse_nfa = compiler
                        .configure(thompson.reverse(true))
                        .build_from_hir(hir)?;
                    let reverse = dense::Builder::new()
                        .configure(
                            dfa_dense
                                .prefilter(None)
                                .specialize_start_states(false)
                                .start_kind(dfa::StartKind::Anchored)
                                .match_kind(regex_automata::MatchKind::All),
                        )
                        .build_from_nfa(&reverse_nfa)
                        .unwrap();

                    dfa::regex::Regex::builder()
                        .build_from_dfas(forward, reverse)
                };
                let cp = cp::Regex::builder()
                    .syntax(syntax)
                    .configure(thompson)
                    .ib(ib)
                    .maybe_ib_parser(ib_parser)
                    .backtrack(backtrack)
                    .build_from_hir(hir)?;
                RegexI::Cp { dfa, cp }
            }
        };

        Ok(Self { imp })
    }

    /// Create a new empty set of capturing groups that is guaranteed to be
    /// valid for the search APIs on this `BoundedBacktracker`.
    ///
    /// A `Captures` value created for a specific `BoundedBacktracker` cannot
    /// be used with any other `BoundedBacktracker`.
    ///
    /// This is a convenience function for [`Captures::all`]. See the
    /// [`Captures`] documentation for an explanation of its alternative
    /// constructors that permit the `BoundedBacktracker` to do less work
    /// during a search, and thus might make it faster.
    pub fn create_captures(&self) -> Captures {
        match &self.imp {
            RegexI::Ib(_) => Captures::matches(GroupInfo::empty()),
            RegexI::Cp { dfa: _, cp } => cp.create_captures(),
        }
    }
}

impl<'a, S: builder::State> Builder<'a, '_, S> {
    /// Configure the syntax options when parsing a pattern string while
    /// building a `Regex`.
    ///
    /// These options _only_ apply when [`Builder::build`] or [`Builder::build_many`]
    /// are used. The other build methods accept `Hir` values, which have
    /// already been parsed.
    ///
    /// # Example
    ///
    /// This example shows how to enable case insensitive mode.
    ///
    /// ```
    /// use ib_matcher::regex::{lita::Regex, util::syntax, Match};
    ///
    /// let re = Regex::builder()
    ///     .syntax(syntax::Config::new().case_insensitive(true))
    ///     .build(r"δ")?;
    /// assert_eq!(Some(Match::must(0, 0..2)), re.find(r"Δ"));
    ///
    /// Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn syntax(mut self, syntax: util::syntax::Config) -> Self {
        self.syntax = syntax;
        self
    }

    /// Builds a `Regex` from a single pattern string.
    ///
    /// If there was a problem parsing the pattern or a problem turning it into
    /// a regex matcher, then an error is returned.
    ///
    /// # Example
    ///
    /// This example shows how to configure syntax options.
    ///
    /// ```
    /// use ib_matcher::regex::{lita::Regex, util::syntax, Match};
    ///
    /// let re = Regex::builder()
    ///     .syntax(syntax::Config::new().crlf(true).multi_line(true))
    ///     .build(r"^foo$")?;
    /// let hay = "\r\nfoo\r\n";
    /// assert_eq!(Some(Match::must(0, 2..5)), re.find(hay));
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn build(self, pattern: &str) -> Result<Regex<'a>, BuildError>
    where
        S::HirAscii: builder::IsUnset,
    {
        let syntax = self.syntax;

        // Parse
        let pattern = pattern.as_ref();
        let parse_with = |syntax| {
            regex_automata::util::syntax::parse_with(pattern, &syntax).map_err(
                |_| {
                    // Shit
                    thompson::Compiler::new()
                        .syntax(syntax)
                        .build(pattern)
                        .unwrap_err()
                },
            )
        };
        let hir_ascii = parse_with(
            syntax
                .unicode(false)
                // ASCII must be valid UTF-8
                .utf8(false),
        )?;
        let hir = parse_with(syntax)?;
        self.hir_ascii(hir_ascii).build_from_hir(hir)
    }
}

/// High level convenience routines for using a regex to search a haystack.
impl<'a> Regex<'a> {
    /// Returns true if and only if this regex matches the given haystack.
    ///
    /// This routine may short circuit if it knows that scanning future input
    /// will never lead to a different result. (Consider how this might make
    /// a difference given the regex `a+` on the haystack `aaaaaaaaaaaaaaa`.
    /// This routine _may_ stop after it sees the first `a`, but routines like
    /// `find` need to continue searching because `+` is greedy by default.)
    ///
    /// # Example
    ///
    /// ```
    /// use ib_matcher::regex::lita::Regex;
    ///
    /// let re = Regex::new("foo[0-9]+bar")?;
    ///
    /// assert!(re.is_match("foo12345bar"));
    /// assert!(!re.is_match("foobar"));
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Example: consistency with search APIs
    ///
    /// `is_match` is guaranteed to return `true` whenever `find` returns a
    /// match. This includes searches that are executed entirely within a
    /// codepoint:
    ///
    /// ```
    /// use ib_matcher::regex::{lita::Regex, Input};
    ///
    /// let re = Regex::new("a*")?;
    ///
    /// // This doesn't match because the default configuration bans empty
    /// // matches from splitting a codepoint.
    /// assert!(!re.is_match(Input::new("☃").span(1..2)));
    /// assert_eq!(None, re.find(Input::new("☃").span(1..2)));
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Notice that when UTF-8 mode is disabled, then the above reports a
    /// match because the restriction against zero-width matches that split a
    /// codepoint has been lifted:
    ///
    /// ```
    /// use ib_matcher::regex::{lita::Regex, Input, Match};
    ///
    /// let re = Regex::builder()
    ///     .thompson(Regex::config().utf8(false))
    ///     .build("a*")?;
    ///
    /// assert!(re.is_match(Input::new("☃").span(1..2)));
    /// assert_eq!(
    ///     Some(Match::must(0, 1..1)),
    ///     re.find(Input::new("☃").span(1..2)),
    /// );
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// A similar idea applies when using line anchors with CRLF mode enabled,
    /// which prevents them from matching between a `\r` and a `\n`.
    ///
    /// ```
    /// use ib_matcher::regex::{lita::Regex, Input, Match};
    ///
    /// let re = Regex::new(r"(?Rm:$)")?;
    /// assert!(!re.is_match(Input::new("\r\n").span(1..1)));
    /// // A regular line anchor, which only considers \n as a
    /// // line terminator, will match.
    /// let re = Regex::new(r"(?m:$)")?;
    /// assert!(re.is_match(Input::new("\r\n").span(1..1)));
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn is_match<'h, I: Into<Input<'h>>>(&self, input: I) -> bool {
        let input = input.into().earliest(true);
        match &self.imp {
            RegexI::Ib(matcher) => {
                matcher.is_match(matcher::input::Input::from_regex(&input))
            }
            RegexI::Cp { dfa, cp } => {
                if input.haystack().is_ascii() {
                    dfa.is_match(input)
                } else {
                    cp.is_match(input)
                }
            }
        }
    }

    /// Executes a leftmost search and returns the first match that is found,
    /// if one exists.
    ///
    /// # Example
    ///
    /// ```
    /// use ib_matcher::regex::{lita::Regex, Match};
    ///
    /// let re = Regex::new("foo[0-9]+")?;
    /// assert_eq!(Some(Match::must(0, 0..8)), re.find("foo12345"));
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn find<'h, I: Into<Input<'h>>>(&self, input: I) -> Option<Match> {
        let input = input.into();
        match &self.imp {
            RegexI::Ib(matcher) => matcher
                .find(matcher::input::Input::from_regex(&input))
                .map(|m| m.offset(input.start()).into()),
            RegexI::Cp { dfa, cp } => {
                if input.haystack().is_ascii() {
                    dfa.find(input)
                } else {
                    cp.find(input)
                }
            }
        }
    }

    /// Executes a leftmost forward search and writes the spans of capturing
    /// groups that participated in a match into the provided [`Captures`]
    /// value. If no match was found, then [`Captures::is_match`] is guaranteed
    /// to return `false`.
    ///
    /// # Example
    ///
    /// ```
    /// use ib_matcher::regex::{lita::Regex, Span};
    ///
    /// let re = Regex::new(r"^([0-9]{4})-([0-9]{2})-([0-9]{2})$")?;
    /// let mut caps = re.create_captures();
    ///
    /// re.captures("2010-03-14", &mut caps);
    /// assert!(caps.is_match());
    /// assert_eq!(Some(Span::from(0..4)), caps.get_group(1));
    /// assert_eq!(Some(Span::from(5..7)), caps.get_group(2));
    /// assert_eq!(Some(Span::from(8..10)), caps.get_group(3));
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn captures<'h, I: Into<Input<'h>>>(
        &self,
        input: I,
        caps: &mut Captures,
    ) -> Result<(), MatchError> {
        let input = input.into();
        match &self.imp {
            RegexI::Ib(matcher) => {
                let slots = caps.slots_mut();
                if let Some(m) =
                    matcher.find(matcher::input::Input::from_regex(&input))
                {
                    let m = m.offset(input.start());
                    slots[0] = NonMaxUsize::new(m.start());
                    slots[1] = NonMaxUsize::new(m.end());
                    caps.set_pattern(Some(PatternID::ZERO));
                } else {
                    caps.set_pattern(None);
                }
                Ok(())
            }
            RegexI::Cp { dfa, cp } => {
                if input.haystack().is_ascii() && !dfa.is_match(input.clone())
                {
                    caps.set_pattern(None);
                    return Ok(());
                }
                cp.captures(input, caps)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use regex_automata::Match;

    use crate::{
        matcher::{PinyinMatchConfig, RomajiMatchConfig},
        pinyin::PinyinNotation,
        syntax::glob,
    };

    use super::*;

    #[test]
    fn empty() {
        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(PinyinMatchConfig::default())
                .build())
            .build("")
            .unwrap();
        assert_eq!(re.find("pyss"), Some(Match::must(0, 0..0)));
        assert_eq!(re.find("apyss"), Some(Match::must(0, 0..0)));
        assert_eq!(re.find("拼音搜索"), Some(Match::must(0, 0..0)));

        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(PinyinMatchConfig::default())
                .is_pattern_partial(true)
                .analyze(true)
                .build())
            .build_from_hir(
                glob::parse_wildcard_path()
                    .separator(glob::PathSeparator::Windows)
                    .call(""),
            )
            .unwrap();
        assert_eq!(re.find("pyss"), Some(Match::must(0, 0..0)));
        assert_eq!(re.find("apyss"), Some(Match::must(0, 0..0)));
        assert_eq!(re.find("拼音搜索"), Some(Match::must(0, 0..0)));
    }

    #[test]
    fn literal() {
        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(PinyinMatchConfig::notations(
                    PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
                ))
                .build())
            .build("pyss")
            .unwrap();

        assert_eq!(re.find("pyss"), Some(Match::must(0, 0..4)));
        assert_eq!(re.find("apyss"), Some(Match::must(0, 1..5)));
        assert_eq!(re.find("拼音搜索"), Some(Match::must(0, 0..12)));

        assert_eq!(re.find("pyss"), Some(Match::must(0, 0..4)));

        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(PinyinMatchConfig::default())
                .is_pattern_partial(true)
                .analyze(true)
                .build())
            .ib_parser(&mut |pattern| Pattern::parse_ev(&pattern).call())
            .build_from_hir(
                glob::parse_wildcard_path()
                    .separator(glob::PathSeparator::Windows)
                    .call("abcdef"),
            )
            .unwrap();
        assert_eq!(re.find("pyss"), None);
        assert_eq!(re.find("abcdef"), Some(Match::must(0, 0..6)));
        assert_eq!(re.find("0abcdef"), Some(Match::must(0, 1..7)));
        assert_eq!(re.find("#文档"), None);
        assert_eq!(re.find("$$"), None);
    }

    #[test]
    fn case() {
        let re = Regex::builder()
            .syntax(util::syntax::Config::new().case_insensitive(true))
            .build(r"δ")
            .unwrap();
        assert_eq!(Some(Match::must(0, 0..2)), re.find(r"Δ"));
    }

    #[test]
    fn alt() {
        let pinyin = PinyinMatchConfig::notations(
            PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
        );

        let re = Regex::builder().build("samwise|sam").unwrap();
        assert_eq!(Some(Match::must(0, 0..3)), re.find("sam"));

        let re = Regex::builder()
            .ib(MatchConfig::builder().pinyin(pinyin.shallow_clone()).build())
            .build("samwise|pyss")
            .unwrap();
        assert_eq!(Some(Match::must(0, 0..12)), re.find("拼音搜索"));
    }

    #[test]
    fn wildcard() {
        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(PinyinMatchConfig::notations(
                    PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
                ))
                .romaji(RomajiMatchConfig::default())
                .build())
            .build("raki.suta")
            .unwrap();

        assert_eq!(re.find("￥らき☆すた"), Some(Match::must(0, 3..18)));

        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(PinyinMatchConfig::notations(
                    PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
                ))
                .build())
            .build("p.*y.*s.*s")
            .unwrap();

        assert_eq!(re.find("拼a音b搜c索d"), Some(Match::must(0, 0..15)));
    }

    #[test]
    fn mix_lang() {
        let pinyin = PinyinMatchConfig::notations(
            PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
        );
        let romaji = RomajiMatchConfig::default();

        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(pinyin.shallow_clone())
                .romaji(romaji.shallow_clone())
                .build())
            .build("pysousuosousounofuri-ren")
            .unwrap();

        assert_eq!(re.find("拼音搜索葬送のフリーレン"), None);

        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(pinyin.shallow_clone())
                .romaji(romaji.shallow_clone())
                .mix_lang(true)
                .build())
            .build("pysousuosousounofuri-ren")
            .unwrap();
        assert_eq!(
            re.find("拼音搜索葬送のフリーレン"),
            Some(Match::must(0, 0..36)),
        );

        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(pinyin.shallow_clone())
                .romaji(romaji.shallow_clone())
                .build())
            .build("(pysousuo)(sousounofuri-ren)")
            .unwrap();

        assert_eq!(
            re.find("拼音搜索葬送のフリーレン"),
            Some(Match::must(0, 0..36)),
        );

        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(pinyin.shallow_clone())
                .romaji(romaji.shallow_clone())
                .build())
            .build("pysousuo.*?sousounofuri-ren")
            .unwrap();

        assert_eq!(
            re.find("拼音搜索⭐葬送のフリーレン"),
            Some(Match::must(0, 0..39)),
        );
    }
}

use std::{
    cell::UnsafeCell,
    marker::PhantomPinned,
    mem::{transmute, MaybeUninit},
    ops::Deref,
    sync::Arc,
};

use bon::bon;
use itertools::Itertools;
use regex_syntax::hir::Hir;

#[cfg(feature = "regex-callback")]
use crate::regex::nfa::Callback;
use crate::{
    matcher::{pattern::Pattern, IbMatcher, MatchConfig},
    regex::{
        nfa::{
            backtrack::{self, BoundedBacktracker},
            thompson::{self},
            NFA,
        },
        syntax,
        util::{self, captures::Captures, pool::Pool},
        Input, Match, MatchError,
    },
};

pub use crate::regex::nfa::{
    backtrack::{Cache, Config, TryCapturesMatches, TryFindMatches},
    thompson::BuildError,
};

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
/// use ib_matcher::regex::cp::Regex;
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
*/
/// For more examples and the syntax, see [`crate::regex`].
///
/// # Case insensitivity
/// To enable case insensitivity:
/// ```
/// use ib_matcher::{matcher::{PinyinMatchConfig, PlainMatchConfig, MatchConfig}, regex::cp::Regex};
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
/**
# Custom matching callbacks
Custom matching callbacks can be used to implement ad hoc look-around, backreferences, balancing groups/recursion/subroutines, combining domain-specific parsers, etc.

Basic usage:
```
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

## Look-around
```
use ib_matcher::regex::cp::Regex;

let re = Regex::builder()
    .callback("lookahead_is_ascii", |input, at, push| {
        let haystack = &input.haystack()[at..];
        if haystack.len() > 0 && haystack[0].is_ascii() {
            push(0);
        }
    })
    .build(r"[\x00-\x7f]+?\d(lookahead_is_ascii)")
    .unwrap();
let hay = "that4Ｕ,this4me1plz";
assert_eq!(
    re.find_iter(hay).map(|m| &hay[m.span()]).collect::<Vec<_>>(),
    vec![",this4", "me1"]
);
```

## Balancing groups
```
use std::{cell::RefCell, rc::Rc};
use ib_matcher::regex::cp::Regex;

let count = Rc::new(RefCell::new(0));
let re = Regex::builder()
    .callback("open_quote", {
        let count = count.clone();
        move |input, at, push| {
            if at < 2 || input.haystack()[at - 2] != b'\\' {
                let mut count = count.borrow_mut();
                *count += 1;
                push(0);
            }
        }
    })
    .callback("close_quote", move |input, at, push| {
        if at < 2 || input.haystack()[at - 2] != b'\\' {
            let mut count = count.borrow_mut();
            if *count > 0 {
                push(0);
            }
            *count -= 1;
        }
    })
    .build(r"'(open_quote).*?'(close_quote)")
    .unwrap();
let hay = r"'one' 'two\'three' 'four'";
assert_eq!(
    re.find_iter(hay).map(|m| &hay[m.span()]).collect::<Vec<_>>(),
    vec!["'one'", r"'two\'three'", "'four'"]
);
```
(In this simple example, just using `'([^'\\]+?|\\')*'` is actually enough, but there are more complex cases where balancing groups (or recursion/subroutines) are necessary.)
*/
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
pub struct Regex<'a> {
    /// The actual regex implementation.
    imp: Arc<RegexI<'a>>,
    /// A thread safe pool of caches.
    ///
    /// For the higher level search APIs, a `Cache` is automatically plucked
    /// from this pool before running a search. The lower level `with` methods
    /// permit the caller to provide their own cache, thereby bypassing
    /// accesses to this pool.
    ///
    /// Note that we put this outside the `Arc` so that cloning a `Regex`
    /// results in creating a fresh `CachePool`. This in turn permits callers
    /// to clone regexes into separate threads where each such regex gets
    /// the pool's "thread owner" optimization. Otherwise, if one shares the
    /// `Regex` directly, then the pool will go through a slower mutex path for
    /// all threads except for the "owner."
    pool: Pool<Cache>,
}

/// The internal implementation of `Regex`, split out so that it can be wrapped
/// in an `Arc`.
struct RegexI<'a> {
    /// The core matching engine.
    re: MaybeUninit<BoundedBacktracker>,
    /// [`IbMatcher`]s in [`NFA`] states may have references to this config due to `shallow_clone()`, i.e. self-references.
    /// We must keep it alive and not move it.
    /// That's also the main reason why we wrap it into `Arc` (the core part of `BoundedBacktracker` is already `Arc`ed).
    config: MatchConfig<'a>,
    _pin: PhantomPinned,
}

/// `Cache::new` doesn't really need `&BoundedBacktracker`, so...
fn create_cache() -> Cache {
    Cache::new(unsafe { &*(8 as *const _) })
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
    /// use ib_matcher::regex::{cp::Regex, util::{syntax, look::LookMatcher}, Match};
    ///
    /// let mut lookm = LookMatcher::new();
    /// lookm.set_line_terminator(b'\x00');
    /// let re = Regex::builder()
    ///     .syntax(syntax::Config::new().multi_line(true))
    ///     .configure(Regex::config().look_matcher(lookm))
    ///     .build(r"^foo$")?;
    /// let hay = "\x00foo\x00";
    /// assert_eq!(Some(Match::must(0, 1..4)), re.find(hay));
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[builder(builder_type = Builder, finish_fn(name = build_many_from_hir, doc {
    /// Builds a `Regex` directly from many `Hir` expressions.
    ///
    /// This is useful if you needed to parse pattern strings into `Hir`
    /// expressions for other reasons (such as analysis or transformations).
    /// This routine permits building a `Regex` directly from the `Hir`
    /// expressions instead of first converting the `Hir` expressions back to
    /// pattern strings.
    ///
    /// When using this method, any options set via [`Builder::syntax`] are
    /// ignored. Namely, the syntax options only apply when parsing a pattern
    /// string, which isn't relevant here.
    ///
    /// If there was a problem building the underlying regex matcher for the
    /// given `Hir` expressions, then an error is returned.
    ///
    /// Note that unlike [`Builder::build_many`], this can only fail as a
    /// result of building the underlying matcher. In that case, there is
    /// no single `Hir` expression that can be isolated as a reason for the
    /// failure. So if this routine fails, it's not possible to determine which
    /// `Hir` expression caused the failure.
    ///
    /// # Example
    ///
    /// This example shows how one can hand-construct multiple `Hir`
    /// expressions and build a single regex from them without doing any
    /// parsing at all.
    ///
    /// ```
    /// use ib_matcher::regex::{
    ///     cp::Regex, Match,
    ///     syntax::hir::{Hir, Look},
    /// };
    ///
    /// // (?Rm)^foo$
    /// let hir1 = Hir::concat(vec![
    ///     Hir::look(Look::StartCRLF),
    ///     Hir::literal("foo".as_bytes()),
    ///     Hir::look(Look::EndCRLF),
    /// ]);
    /// // (?Rm)^bar$
    /// let hir2 = Hir::concat(vec![
    ///     Hir::look(Look::StartCRLF),
    ///     Hir::literal("bar".as_bytes()),
    ///     Hir::look(Look::EndCRLF),
    /// ]);
    /// let re = Regex::builder()
    ///     .build_many_from_hir(vec![hir1, hir2])?;
    /// let hay = "\r\nfoo\r\nbar";
    /// let got: Vec<Match> = re.find_iter(hay).collect();
    /// let expected = vec![
    ///     Match::must(0, 2..5),
    ///     Match::must(1, 7..10),
    /// ];
    /// assert_eq!(expected, got);
    ///
    /// Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    }))]
    pub fn builder(
        #[builder(field)] syntax: util::syntax::Config,
        #[cfg(feature = "regex-callback")]
        #[builder(field)]
        callbacks: Vec<(String, Callback)>,
        #[builder(finish_fn)] hirs: Vec<Hir>,
        /// Thompson NFA config. Named `configure` to be compatible with [`regex_automata::meta::Builder`]. Although some fields are not supported and `utf8_empty` is named as `utf8` instead.
        #[builder(default)]
        configure: thompson::Config,
        /// [`IbMatcher`] config.
        #[builder(default = MatchConfig::builder().case_insensitive(false).build())]
        ib: MatchConfig<'a>,
        /// `IbMatcher` pattern parser.
        ///
        /// ### Example
        /// ```
        /// use ib_matcher::{regex::cp::Regex, matcher::{MatchConfig, pattern::Pattern}};
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
        dbg!(&hirs);

        let mut imp = Arc::new(RegexI {
            re: MaybeUninit::uninit(),
            config: {
                let mut config = ib;
                config.starts_with = true;
                config
            },
            _pin: PhantomPinned,
        });

        // Copy-and-patch NFA
        let (hirs, literals) =
            syntax::fold::fold_literal_utf8(hirs.into_iter());
        let mut nfa: NFA = thompson::Compiler::new()
            .configure(configure)
            .build_many_from_hir(&hirs)?
            .into();
        let count = literals.len();
        #[cfg(feature = "regex-callback")]
        let count = {
            let mut count = count;
            for (literal, callback) in callbacks {
                for i in literals.iter().positions(|l| l == &literal) {
                    nfa.patch_first_byte(i as u8, |next| {
                        crate::regex::nfa::State::Callback {
                            callback: callback.clone(),
                            next,
                        }
                    });
                    count -= 1;
                }
            }
            count
        };
        nfa.patch_bytes_to_matchers(literals.len() as u8, count, |b| {
            let pattern = literals[b as usize].as_str();
            let pattern = if let Some(ib_parser) = ib_parser.as_mut() {
                ib_parser(pattern)
            } else {
                pattern.into()
            };

            // `shallow_clone()` requires `config` cannot be moved
            let config: MatchConfig<'static> =
                unsafe { transmute(imp.config.shallow_clone()) };
            IbMatcher::with_config(pattern, config)
        });
        #[cfg(test)]
        dbg!(&nfa);

        // Engine
        let re = BoundedBacktracker::builder()
            .configure(backtrack)
            .build_from_nfa(nfa)?;
        unsafe { Arc::get_mut(&mut imp).unwrap_unchecked().re.write(re) };

        Ok(Self { imp, pool: Pool::new(create_cache) })
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
    /// use ib_matcher::regex::{cp::Regex, util::syntax, Match};
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

    /// Add a [custom matching callback](Regex#custom-matching-callbacks).
    #[cfg(feature = "regex-callback")]
    pub fn callback(
        mut self,
        literal: impl Into<String>,
        callback: impl Fn(&Input, usize, &mut dyn FnMut(usize)) + 'static,
    ) -> Self {
        self.callbacks.push((literal.into(), Arc::new(callback)));
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
    /// use ib_matcher::regex::{cp::Regex, util::syntax, Match};
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
        S: builder::IsComplete,
    {
        self.build_many(&[pattern])
    }

    /// Builds a `Regex` from many pattern strings.
    ///
    /// If there was a problem parsing any of the patterns or a problem turning
    /// them into a regex matcher, then an error is returned.
    ///
    /// # Example: zero patterns is valid
    ///
    /// Building a regex with zero patterns results in a regex that never
    /// matches anything. Because this routine is generic, passing an empty
    /// slice usually requires a turbo-fish (or something else to help type
    /// inference).
    ///
    /// ```
    /// use ib_matcher::regex::{cp::Regex, util::syntax, Match};
    ///
    /// let re = Regex::builder()
    ///     .build_many::<&str>(&[])?;
    /// assert_eq!(None, re.find(""));
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn build_many<P: AsRef<str>>(
        self,
        patterns: &[P],
    ) -> Result<Regex<'a>, BuildError>
    where
        S: builder::IsComplete,
    {
        // Bypass case_fold_char()
        // case_insensitive class and (?i) will be broken
        // .case_insensitive(false)
        let syntax = self.syntax;

        // Parse
        let hirs = patterns
            .into_iter()
            .map(|pattern| {
                let pattern = pattern.as_ref();
                regex_automata::util::syntax::parse_with(pattern, &syntax)
                    .map_err(|_| {
                        // Shit
                        thompson::Compiler::new()
                            .syntax(syntax)
                            .build(pattern)
                            .unwrap_err()
                    })
            })
            .try_collect()?;
        self.build_many_from_hir(hirs)
    }

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
    ///     cp::Regex, Match,
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
    pub fn build_from_hir(self, hir: Hir) -> Result<Regex<'a>, BuildError>
    where
        S: builder::IsComplete,
    {
        self.build_many_from_hir(vec![hir])
    }
}

impl Clone for Regex<'_> {
    fn clone(&self) -> Self {
        Regex { imp: self.imp.clone(), pool: Pool::new(create_cache) }
    }
}

impl Drop for RegexI<'_> {
    fn drop(&mut self) {
        unsafe { self.re.assume_init_drop() };
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
    /// use ib_matcher::regex::cp::Regex;
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
    /// use ib_matcher::regex::{cp::Regex, Input};
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
    /// use ib_matcher::regex::{cp::Regex, Input, Match};
    ///
    /// let re = Regex::builder()
    ///     .configure(Regex::config().utf8(false))
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
    /// use ib_matcher::regex::{cp::Regex, Input, Match};
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
        let mut guard = self.pool.get();
        self.try_is_match(&mut guard, input).unwrap()
    }

    /// Executes a leftmost search and returns the first match that is found,
    /// if one exists.
    ///
    /// # Example
    ///
    /// ```
    /// use ib_matcher::regex::{cp::Regex, Match};
    ///
    /// let re = Regex::new("foo[0-9]+")?;
    /// assert_eq!(Some(Match::must(0, 0..8)), re.find("foo12345"));
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn find<'h, I: Into<Input<'h>>>(&self, input: I) -> Option<Match> {
        let input = input.into();
        let mut guard = self.pool.get();
        self.try_find(&mut guard, input).unwrap()
    }

    /// Executes a leftmost forward search and writes the spans of capturing
    /// groups that participated in a match into the provided [`Captures`]
    /// value. If no match was found, then [`Captures::is_match`] is guaranteed
    /// to return `false`.
    ///
    /// # Example
    ///
    /// ```
    /// use ib_matcher::regex::{cp::Regex, Span};
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
        let mut guard = self.pool.get();
        self.try_captures(&mut guard, input, caps)
    }

    /// Returns an iterator over all non-overlapping leftmost matches in
    /// the given haystack. If no match exists, then the iterator yields no
    /// elements.
    ///
    /// # Example
    ///
    /// ```
    /// use ib_matcher::regex::{cp::Regex, Match};
    ///
    /// let re = Regex::new("foo[0-9]+")?;
    /// let haystack = "foo1 foo12 foo123";
    /// let matches: Vec<Match> = re.find_iter(haystack).collect();
    /// assert_eq!(matches, vec![
    ///     Match::must(0, 0..4),
    ///     Match::must(0, 5..10),
    ///     Match::must(0, 11..17),
    /// ]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn find_iter<'h, I: Into<Input<'h>>>(
        &'h self,
        input: I,
    ) -> impl Iterator<Item = Match> + 'h {
        let input = input.into();
        let guard = UnsafeCell::new(self.pool.get());
        self.try_find_iter(unsafe { &mut *guard.get() }, input).map(move |r| {
            let _guard = &guard;
            r.unwrap()
        })
    }

    /// Returns an iterator over all non-overlapping `Captures` values. If no
    /// match exists, then the iterator yields no elements.
    ///
    /// This yields the same matches as [`Regex::find_iter`], but it includes
    /// the spans of all capturing groups that participate in each match.
    ///
    /// **Tip:** See [`util::iter::Searcher`](crate::util::iter::Searcher) for
    /// how to correctly iterate over all matches in a haystack while avoiding
    /// the creation of a new `Captures` value for every match. (Which you are
    /// forced to do with an `Iterator`.)
    ///
    /// # Example
    ///
    /// ```
    /// use ib_matcher::regex::{cp::Regex, Span};
    ///
    /// let re = Regex::new("foo(?P<numbers>[0-9]+)")?;
    ///
    /// let haystack = "foo1 foo12 foo123";
    /// let matches: Vec<Span> = re
    ///     .captures_iter(haystack)
    ///     // The unwrap is OK since 'numbers' matches if the pattern matches.
    ///     .map(|caps| caps.get_group_by_name("numbers").unwrap())
    ///     .collect();
    /// assert_eq!(matches, vec![
    ///     Span::from(3..4),
    ///     Span::from(8..10),
    ///     Span::from(14..17),
    /// ]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn captures_iter<'h, I: Into<Input<'h>>>(
        &'h self,
        input: I,
    ) -> impl Iterator<Item = Captures> + 'h {
        let input = input.into();
        let guard = UnsafeCell::new(self.pool.get());
        self.try_captures_iter(unsafe { &mut *guard.get() }, input).map(
            move |r| {
                let _guard = &guard;
                r.unwrap()
            },
        )
    }
}

impl Deref for Regex<'_> {
    type Target = BoundedBacktracker;

    fn deref(&self) -> &Self::Target {
        unsafe { self.imp.re.assume_init_ref() }
    }
}

#[cfg(test)]
mod tests {
    use regex_automata::Match;

    use crate::{
        matcher::{PinyinMatchConfig, RomajiMatchConfig},
        pinyin::PinyinNotation,
    };

    use super::*;

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

        let mut cache = re.create_cache();
        assert_eq!(
            re.try_find(&mut cache, "pyss").unwrap(),
            Some(Match::must(0, 0..4)),
        );
        assert_eq!(
            re.try_find(&mut cache, "apyss").unwrap(),
            Some(Match::must(0, 1..5)),
        );
        assert_eq!(
            re.try_find(&mut cache, "拼音搜索").unwrap(),
            Some(Match::must(0, 0..12)),
        );

        assert_eq!(re.find("pyss"), Some(Match::must(0, 0..4)),);
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

        assert_eq!(re.max_haystack_len(), 0x1111111111111110);
        let mut cache = re.create_cache();
        assert_eq!(cache.memory_usage(), 0);
        assert_eq!(
            re.try_find(&mut cache, "￥らき☆すた").unwrap(),
            Some(Match::must(0, 3..18)),
        );
        // 2 * 16 + (alignup(16 * (18+1) / 8, 8) = 40)
        assert_eq!(cache.memory_usage(), 72);

        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(PinyinMatchConfig::notations(
                    PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
                ))
                .build())
            .build("p.*y.*s.*s")
            .unwrap();
        let mut cache = re.create_cache();
        assert_eq!(
            re.try_find(&mut cache, "拼a音b搜c索d").unwrap(),
            Some(Match::must(0, 0..15)),
        );
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
        let mut cache = re.create_cache();
        assert_eq!(
            re.try_find(&mut cache, "拼音搜索葬送のフリーレン").unwrap(),
            None
        );

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
        let mut cache = re.create_cache();
        assert_eq!(
            re.try_find(&mut cache, "拼音搜索葬送のフリーレン").unwrap(),
            Some(Match::must(0, 0..36)),
        );

        let re = Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(pinyin.shallow_clone())
                .romaji(romaji.shallow_clone())
                .build())
            .build("pysousuo.*?sousounofuri-ren")
            .unwrap();
        let mut cache = re.create_cache();
        assert_eq!(
            re.try_find(&mut cache, "拼音搜索⭐葬送のフリーレン").unwrap(),
            Some(Match::must(0, 0..39)),
        );
    }

    #[cfg(feature = "regex-callback")]
    #[test]
    fn callback() {
        use std::{cell::RefCell, rc::Rc};

        let re = Regex::builder()
            .callback("ascii", |input, at, push| {
                let haystack = &input.haystack()[at..];
                if haystack.get(0).is_some_and(|c| c.is_ascii()) {
                    push(1);
                }
            })
            .build(r"(ascii)+\d(ascii)+")
            .unwrap();
        assert_eq!(re.find("that4Ｕ this4me"), Some(Match::must(0, 8..16)));

        let count = Rc::new(RefCell::new(0));
        let re = Regex::builder()
            .callback("open_quote", {
                let count = count.clone();
                move |input, at, push| {
                    if at < 2 || input.haystack()[at - 2] != b'\\' {
                        let mut count = count.borrow_mut();
                        *count += 1;
                        push(0);
                    }
                }
            })
            .callback("close_quote", move |input, at, push| {
                if at < 2 || input.haystack()[at - 2] != b'\\' {
                    let mut count = count.borrow_mut();
                    if *count > 0 {
                        push(0);
                    }
                    *count -= 1;
                }
            })
            // '([^'\\]+?|\\')*'
            .build(r"'(open_quote).*?'(close_quote)")
            .unwrap();
        let hay = r"'one' 'two\'three' 'four'";
        assert_eq!(
            re.find_iter(hay).map(|m| &hay[m.span()]).collect::<Vec<_>>(),
            vec!["'one'", r"'two\'three'", "'four'"]
        );

        let re = Regex::builder()
            .callback("lookahead_is_ascii", |input, at, push| {
                let haystack = &input.haystack()[at..];
                if haystack.get(0).is_some_and(|c| c.is_ascii()) {
                    push(0);
                }
            })
            .build(r"(?-u)[\x00-\x7f]+?\d(lookahead_is_ascii)")
            .unwrap();
        let hay = "that4Ｕ,this4me1plz";
        assert_eq!(
            re.find_iter(hay).map(|m| &hay[m.span()]).collect::<Vec<_>>(),
            vec![",this4", "me1"]
        );
    }
}

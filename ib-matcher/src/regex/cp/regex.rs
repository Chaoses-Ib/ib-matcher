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

use crate::{
    matcher::{IbMatcher, MatchConfig},
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
///
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
///
/// # Example: anchored search
///
/// This example shows how to use [`Input::anchored`] to run an anchored
/// search, even when the regex pattern itself isn't anchored. An anchored
/// search guarantees that if a match is found, then the start offset of the
/// match corresponds to the offset at which the search was started.
///
/// ```
/// use ib_matcher::regex::{cp::Regex, Anchored, Input, Match};
///
/// let re = Regex::new(r"\bfoo\b")?;
/// let input = Input::new("xx foo xx").range(3..).anchored(Anchored::Yes);
/// // The offsets are in terms of the original haystack.
/// assert_eq!(Some(Match::must(0, 3..6)), re.find(input));
///
/// // Notice that no match occurs here, because \b still takes the
/// // surrounding context into account, even if it means looking back
/// // before the start of your search.
/// let hay = "xxfoo xx";
/// let input = Input::new(hay).range(2..).anchored(Anchored::Yes);
/// assert_eq!(None, re.find(input));
/// // Indeed, you cannot achieve the above by simply slicing the
/// // haystack itself, since the regex engine can't see the
/// // surrounding context. This is why 'Input' permits setting
/// // the bounds of a search!
/// let input = Input::new(&hay[2..]).anchored(Anchored::Yes);
/// // WRONG!
/// assert_eq!(Some(Match::must(0, 0..3)), re.find(input));
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Example: earliest search
///
/// This example shows how to use [`Input::earliest`] to run a search that
/// might stop before finding the typical leftmost match.
///
/// ```ignore
/// use ib_matcher::regex::{cp::Regex, Anchored, Input, Match};
///
/// let re = Regex::new(r"[a-z]{3}|b")?;
/// let input = Input::new("abc").earliest(true);
/// assert_eq!(Some(Match::must(0, 1..2)), re.find(input));
///
/// // Note that "earliest" isn't really a match semantic unto itself.
/// // Instead, it is merely an instruction to whatever regex engine
/// // gets used internally to quit as soon as it can. For example,
/// // this regex uses a different search technique, and winds up
/// // producing a different (but valid) match!
/// let re = Regex::new(r"abc|b")?;
/// let input = Input::new("abc").earliest(true);
/// assert_eq!(Some(Match::must(0, 0..3)), re.find(input));
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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
        #[builder(finish_fn)] hirs: Vec<Hir>,
        /// Thompson NFA config. Named `configure` to be compatible with [`regex_automata::meta::Builder`]. Although some fields are not supported and `utf8_empty` is named as `utf8` instead.
        #[builder(default)]
        configure: thompson::Config,
        /// [`IbMatcher`] config.
        #[builder(default)]
        ib: MatchConfig<'a>,
        #[builder(default = backtrack::Config::new().visited_capacity(usize::MAX / 8))]
        backtrack: backtrack::Config,
    ) -> Result<Self, BuildError> {
        _ = syntax;
        #[cfg(test)]
        dbg!(&hirs);

        let mut imp = Arc::new(RegexI {
            re: MaybeUninit::uninit(),
            config: ib,
            _pin: PhantomPinned,
        });

        // Copy-and-patch NFA
        let (hirs, literals) =
            syntax::fold::fold_literal_utf8(hirs.into_iter());
        let mut nfa: NFA = thompson::Compiler::new()
            .configure(configure)
            .build_many_from_hir(&hirs)?
            .into();
        nfa.patch_bytes_to_matchers(literals.len() as u8, |b| {
            // `shallow_clone()` requires `config` cannot be moved
            let config: MatchConfig<'static> =
                unsafe { transmute(imp.config.shallow_clone()) };
            IbMatcher::with_config(literals[b as usize].as_str(), config)
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

impl<'a, S: builder::State> Builder<'a, S> {
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
        // Parse
        let hirs = patterns
            .into_iter()
            .map(|pattern| {
                let pattern = pattern.as_ref();
                regex_automata::util::syntax::parse_with(pattern, &self.syntax)
                    .map_err(|_| {
                        // Shit
                        thompson::Compiler::new()
                            .syntax(self.syntax)
                            .build(pattern)
                            .unwrap_err()
                    })
            })
            .try_collect()?;
        self.build_many_from_hir(hirs)
    }

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
}

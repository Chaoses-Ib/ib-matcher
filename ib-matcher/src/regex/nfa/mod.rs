use core::ops::Deref;
use std::{fmt::Debug, sync::Arc};

use itertools::Itertools;
#[cfg(feature = "syntax-regex")]
use regex_automata::nfa::thompson::BuildError;
use regex_automata::util::primitives::StateID;
#[cfg(feature = "regex-callback")]
use regex_automata::Input;

use crate::matcher::IbMatcher;

pub mod backtrack;

pub use regex_automata::nfa::thompson;

/// A byte oriented Thompson non-deterministic finite automaton (NFA).
///
/// A Thompson NFA is a finite state machine that permits unconditional epsilon
/// transitions, but guarantees that there exists at most one non-epsilon
/// transition for each element in the alphabet for each state.
///
/// An NFA may be used directly for searching, for analysis or to build
/// a deterministic finite automaton (DFA).
///
/// # Cheap clones
///
/// Since an NFA is a core data type in this crate that many other regex
/// engines are based on top of, it is convenient to give ownership of an NFA
/// to said regex engines. Because of this, an NFA uses reference counting
/// internally. Therefore, it is cheap to clone and it is encouraged to do so.
///
/// # Capabilities
///
/// Using an NFA for searching via the
/// [`PikeVM`](crate::nfa::thompson::pikevm::PikeVM) provides the most amount
/// of "power" of any regex engine in this crate. Namely, it supports the
/// following in all cases:
///
/// 1. Detection of a match.
/// 2. Location of a match, including both the start and end offset, in a
/// single pass of the haystack.
/// 3. Location of matching capturing groups.
/// 4. Handles multiple patterns, including (1)-(3) when multiple patterns are
/// present.
///
/// # Capturing Groups
///
/// Groups refer to parenthesized expressions inside a regex pattern. They look
/// like this, where `exp` is an arbitrary regex:
///
/// * `(exp)` - An unnamed capturing group.
/// * `(?P<name>exp)` or `(?<name>exp)` - A named capturing group.
/// * `(?:exp)` - A non-capturing group.
/// * `(?i:exp)` - A non-capturing group that sets flags.
///
/// Only the first two forms are said to be _capturing_. Capturing
/// means that the last position at which they match is reportable. The
/// [`Captures`](crate::util::captures::Captures) type provides convenient
/// access to the match positions of capturing groups, which includes looking
/// up capturing groups by their name.
///
/// # Byte oriented
///
/// This NFA is byte oriented, which means that all of its transitions are
/// defined on bytes. In other words, the alphabet of an NFA consists of the
/// 256 different byte values.
///
/// While DFAs nearly demand that they be byte oriented for performance
/// reasons, an NFA could conceivably be *Unicode codepoint* oriented. Indeed,
/// a previous version of this NFA supported both byte and codepoint oriented
/// modes. A codepoint oriented mode can work because an NFA fundamentally uses
/// a sparse representation of transitions, which works well with the large
/// sparse space of Unicode codepoints.
///
/// Nevertheless, this NFA is only byte oriented. This choice is primarily
/// driven by implementation simplicity, and also in part memory usage. In
/// practice, performance between the two is roughly comparable. However,
/// building a DFA (including a hybrid DFA) really wants a byte oriented NFA.
/// So if we do have a codepoint oriented NFA, then we also need to generate
/// byte oriented NFA in order to build an hybrid NFA/DFA. Thus, by only
/// generating byte oriented NFAs, we can produce one less NFA. In other words,
/// if we made our NFA codepoint oriented, we'd need to *also* make it support
/// a byte oriented mode, which is more complicated. But a byte oriented mode
/// can support everything.
///
/// # Differences with DFAs
///
/// At the theoretical level, the precise difference between an NFA and a DFA
/// is that, in a DFA, for every state, an input symbol unambiguously refers
/// to a single transition _and_ that an input symbol is required for each
/// transition. At a practical level, this permits DFA implementations to be
/// implemented at their core with a small constant number of CPU instructions
/// for each byte of input searched. In practice, this makes them quite a bit
/// faster than NFAs _in general_. Namely, in order to execute a search for any
/// Thompson NFA, one needs to keep track of a _set_ of states, and execute
/// the possible transitions on all of those states for each input symbol.
/// Overall, this results in much more overhead. To a first approximation, one
/// can expect DFA searches to be about an order of magnitude faster.
///
/// So why use an NFA at all? The main advantage of an NFA is that it takes
/// linear time (in the size of the pattern string after repetitions have been
/// expanded) to build and linear memory usage. A DFA, on the other hand, may
/// take exponential time and/or space to build. Even in non-pathological
/// cases, DFAs often take quite a bit more memory than their NFA counterparts,
/// _especially_ if large Unicode character classes are involved. Of course,
/// an NFA also provides additional capabilities. For example, it can match
/// Unicode word boundaries on non-ASCII text and resolve the positions of
/// capturing groups.
///
/// Note that a [`hybrid::regex::Regex`](crate::hybrid::regex::Regex) strikes a
/// good balance between an NFA and a DFA. It avoids the exponential build time
/// of a DFA while maintaining its fast search time. The downside of a hybrid
/// NFA/DFA is that in some cases it can be slower at search time than the NFA.
/// (It also has less functionality than a pure NFA. It cannot handle Unicode
/// word boundaries on non-ASCII text and cannot resolve capturing groups.)
///
/// # Example
///
/// This shows how to build an NFA with the default configuration and execute a
/// search using the Pike VM.
///
/// ```
/// use regex_automata::{nfa::thompson::pikevm::PikeVM, Match};
///
/// let re = PikeVM::new(r"foo[0-9]+")?;
/// let mut cache = re.create_cache();
/// let mut caps = re.create_captures();
///
/// let expected = Some(Match::must(0, 0..8));
/// re.captures(&mut cache, b"foo12345", &mut caps);
/// assert_eq!(expected, caps.get_match());
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Example: resolving capturing groups
///
/// This example shows how to parse some simple dates and extract the
/// components of each date via capturing groups.
///
/// ```
/// # if cfg!(miri) { return Ok(()); } // miri takes too long
/// use regex_automata::{
///     nfa::thompson::pikevm::PikeVM,
///     util::captures::Captures,
/// };
///
/// let vm = PikeVM::new(r"(?P<y>\d{4})-(?P<m>\d{2})-(?P<d>\d{2})")?;
/// let mut cache = vm.create_cache();
///
/// let haystack = "2012-03-14, 2013-01-01 and 2014-07-05";
/// let all: Vec<Captures> = vm.captures_iter(
///     &mut cache, haystack.as_bytes()
/// ).collect();
/// // There should be a total of 3 matches.
/// assert_eq!(3, all.len());
/// // The year from the second match is '2013'.
/// let span = all[1].get_group_by_name("y").unwrap();
/// assert_eq!("2013", &haystack[span]);
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// This example shows that only the last match of a capturing group is
/// reported, even if it had to match multiple times for an overall match
/// to occur.
///
/// ```
/// use regex_automata::{nfa::thompson::pikevm::PikeVM, Span};
///
/// let re = PikeVM::new(r"([a-z]){4}")?;
/// let mut cache = re.create_cache();
/// let mut caps = re.create_captures();
///
/// let haystack = b"quux";
/// re.captures(&mut cache, haystack, &mut caps);
/// assert!(caps.is_match());
/// assert_eq!(Some(Span::from(3..4)), caps.get_group(1));
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct NFA(
    // We make NFAs reference counted primarily for two reasons. First is that
    // the NFA type itself is quite large (at least 0.5KB), and so it makes
    // sense to put it on the heap by default anyway. Second is that, for Arc
    // specifically, this enables cheap clones. This tends to be useful because
    // several structures (the backtracker, the Pike VM, the hybrid NFA/DFA)
    // all want to hang on to an NFA for use during search time. We could
    // provide the NFA at search time via a function argument, but this makes
    // for an unnecessarily annoying API. Instead, we just let each structure
    // share ownership of the NFA. Using a deep clone would not be smart, since
    // the NFA can use quite a bit of heap space.
    Arc<Inner>,
);

impl From<thompson::NFA> for NFA {
    fn from(nfa: thompson::NFA) -> Self {
        let states = nfa.states().iter().cloned().map_into().collect();
        Self(Inner { nfa, states }.into())
    }
}

impl NFA {
    /// Parse the given regular expression using a default configuration and
    /// build an NFA from it.
    ///
    /// If you want a non-default configuration, then use the NFA
    /// [`Compiler`] with a [`Config`].
    ///
    /// # Example
    ///
    /// ```
    /// use regex_automata::{nfa::thompson::pikevm::PikeVM, Match};
    ///
    /// let re = PikeVM::new(r"foo[0-9]+")?;
    /// let (mut cache, mut caps) = (re.create_cache(), re.create_captures());
    ///
    /// let expected = Some(Match::must(0, 0..8));
    /// re.captures(&mut cache, b"foo12345", &mut caps);
    /// assert_eq!(expected, caps.get_match());
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[cfg(feature = "syntax-regex")]
    pub fn new(pattern: &str) -> Result<NFA, BuildError> {
        thompson::NFA::compiler().build(pattern).map(Into::into)
    }

    /// Parse the given regular expressions using a default configuration and
    /// build a multi-NFA from them.
    ///
    /// If you want a non-default configuration, then use the NFA
    /// [`Compiler`] with a [`Config`].
    ///
    /// # Example
    ///
    /// ```
    /// use regex_automata::{nfa::thompson::pikevm::PikeVM, Match};
    ///
    /// let re = PikeVM::new_many(&["[0-9]+", "[a-z]+"])?;
    /// let (mut cache, mut caps) = (re.create_cache(), re.create_captures());
    ///
    /// let expected = Some(Match::must(1, 0..3));
    /// re.captures(&mut cache, b"foo12345bar", &mut caps);
    /// assert_eq!(expected, caps.get_match());
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[cfg(feature = "syntax-regex")]
    pub fn new_many<P: AsRef<str>>(patterns: &[P]) -> Result<NFA, BuildError> {
        thompson::NFA::compiler().build_many(patterns).map(Into::into)
    }

    /// Returns an NFA with a single regex pattern that always matches at every
    /// position.
    ///
    /// # Example
    ///
    /// ```
    /// use regex_automata::{nfa::thompson::{NFA, pikevm::PikeVM}, Match};
    ///
    /// let re = PikeVM::new_from_nfa(NFA::always_match())?;
    /// let (mut cache, mut caps) = (re.create_cache(), re.create_captures());
    ///
    /// let expected = Some(Match::must(0, 0..0));
    /// re.captures(&mut cache, b"", &mut caps);
    /// assert_eq!(expected, caps.get_match());
    /// re.captures(&mut cache, b"foo", &mut caps);
    /// assert_eq!(expected, caps.get_match());
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn always_match() -> NFA {
        thompson::NFA::always_match().into()
    }

    /// Returns an NFA that never matches at any position.
    ///
    /// This is a convenience routine for creating an NFA with zero patterns.
    ///
    /// # Example
    ///
    /// ```
    /// use regex_automata::nfa::thompson::{NFA, pikevm::PikeVM};
    ///
    /// let re = PikeVM::new_from_nfa(NFA::never_match())?;
    /// let (mut cache, mut caps) = (re.create_cache(), re.create_captures());
    ///
    /// re.captures(&mut cache, b"", &mut caps);
    /// assert!(!caps.is_match());
    /// re.captures(&mut cache, b"foo", &mut caps);
    /// assert!(!caps.is_match());
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn never_match() -> NFA {
        thompson::NFA::never_match().into()
    }

    /// Return a reference to the NFA state corresponding to the given ID.
    ///
    /// This is a convenience routine for `nfa.states()[id]`.
    ///
    /// # Panics
    ///
    /// This panics when the given identifier does not reference a valid state.
    /// That is, when `id.as_usize() >= nfa.states().len()`.
    ///
    /// # Example
    ///
    /// The anchored state for a pattern will typically correspond to a
    /// capturing state for that pattern. (Although, this is not an API
    /// guarantee!)
    ///
    /// ```
    /// use regex_automata::{nfa::thompson::{NFA, State}, PatternID};
    ///
    /// let nfa = NFA::new("a")?;
    /// let state = nfa.state(nfa.start_pattern(PatternID::ZERO).unwrap());
    /// match *state {
    ///     State::Capture { slot, .. } => {
    ///         assert_eq!(0, slot.as_usize());
    ///     }
    ///     _ => unreachable!("unexpected state"),
    /// }
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn state(&self, id: StateID) -> &State {
        &self.states()[id]
    }

    /// Returns a slice of all states in this NFA.
    ///
    /// The slice returned is indexed by `StateID`. This provides a convenient
    /// way to access states while following transitions among those states.
    ///
    /// # Example
    ///
    /// This demonstrates that disabling UTF-8 mode can shrink the size of the
    /// NFA considerably in some cases, especially when using Unicode character
    /// classes.
    ///
    /// ```
    /// # if cfg!(miri) { return Ok(()); } // miri takes too long
    /// use regex_automata::nfa::thompson::NFA;
    ///
    /// let nfa_unicode = NFA::new(r"\w")?;
    /// let nfa_ascii = NFA::new(r"(?-u)\w")?;
    /// // Yes, a factor of 45 difference. No lie.
    /// assert!(40 * nfa_ascii.states().len() < nfa_unicode.states().len());
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn states(&self) -> &[State] {
        &self.0.states
    }
}

impl Deref for NFA {
    type Target = thompson::NFA;

    fn deref(&self) -> &Self::Target {
        &self.0.nfa
    }
}

#[derive(Debug)]
pub(super) struct Inner {
    nfa: thompson::NFA,
    /// The state sequence. This sequence is guaranteed to be indexable by all
    /// starting state IDs, and it is also guaranteed to contain at most one
    /// `Match` state for each pattern compiled into this NFA. (A pattern may
    /// not have a corresponding `Match` state if a `Match` state is impossible
    /// to reach.)
    states: Vec<State>,
}

#[cfg(feature = "regex-callback")]
pub type Callback = Arc<dyn Fn(&Input, usize, &mut dyn FnMut(usize))>;

/// A state in an NFA.
///
/// In theory, it can help to conceptualize an `NFA` as a graph consisting of
/// `State`s. Each `State` contains its complete set of outgoing transitions.
///
/// In practice, it can help to conceptualize an `NFA` as a sequence of
/// instructions for a virtual machine. Each `State` says what to do and where
/// to go next.
///
/// Strictly speaking, the practical interpretation is the most correct one,
/// because of the [`Capture`](State::Capture) state. Namely, a `Capture`
/// state always forwards execution to another state unconditionally. Its only
/// purpose is to cause a side effect: the recording of the current input
/// position at a particular location in memory. In this sense, an `NFA`
/// has more power than a theoretical non-deterministic finite automaton.
///
/// For most uses of this crate, it is likely that one may never even need to
/// be aware of this type at all. The main use cases for looking at `State`s
/// directly are if you need to write your own search implementation or if you
/// need to do some kind of analysis on the NFA.
// Clone, Eq, PartialEq
pub enum State {
    Nfa(thompson::State),
    IbMatcher {
        matcher: IbMatcher<'static>,
        next: StateID,
    },
    #[cfg(feature = "regex-callback")]
    Callback {
        callback: Callback,
        next: StateID,
    },
}

impl From<thompson::State> for State {
    fn from(state: thompson::State) -> Self {
        State::Nfa(state)
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Nfa(state) => write!(f, "Nfa({:?})", state),
            State::IbMatcher { matcher, next } => {
                write!(f, "IbMatcher({:?}, {:?})", matcher, next)
            }
            #[cfg(feature = "regex-callback")]
            State::Callback { next, .. } => {
                write!(f, "Callback({:?})", next)
            }
        }
    }
}

impl NFA {
    pub fn states_mut(&mut self) -> &mut Vec<State> {
        &mut Arc::get_mut(&mut self.0).unwrap().states
    }

    pub fn patch_first_byte(
        &mut self,
        byte: u8,
        state: impl FnOnce(StateID) -> State,
    ) {
        for s in self.states_mut() {
            match *s {
                State::Nfa(thompson::State::ByteRange {
                    trans: thompson::Transition { start, end, next },
                }) if start == byte && end == byte => {
                    *s = state(next);
                    break;
                }
                _ => (),
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn patch_first_byte_to_matcher(
        &mut self,
        byte: u8,
        matcher: IbMatcher<'static>,
    ) {
        self.patch_first_byte(byte, |next| State::IbMatcher { matcher, next })
    }

    pub(crate) fn count_bytes(&self, lt: u8) -> usize {
        self.states()
            .iter()
            .filter(|s| {
                matches!(s, State::Nfa(thompson::State::ByteRange { trans: thompson::Transition { start, end, .. } }) if start == end && *start < lt)
            })
            .count()
    }

    /// [`thompson::State::ByteRange`] may come from `c_literal()`, `c_alt_slice()` and [`thompson::State::Sparse`]:
    /// - `c_literal()` and `c_alt_slice()` can be controlled by [`crate::regex::syntax::fold::fold_literal()`]
    /// - `Sparse` may come from:
    ///   - `c_byte_class()` (only if `build_from_hir()`)
    ///   - `Utf8Compiler` from `c_unicode_class()` with non-ASCII class. Its byte is presumably always larger than 0x7F.
    ///   - `LiteralTrie` from `c_alt_slice()`
    ///
    /// Limit fold to the first 128 literals? Unfolded literals are more prone to conflict; folded literals at least only conflict if there are Unicode classes.
    pub(crate) fn patch_bytes_to_matchers(
        &mut self,
        lt: u8,
        count: usize,
        mut matcher: impl FnMut(u8) -> IbMatcher<'static>,
    ) {
        debug_assert_eq!(self.count_bytes(lt), count, "Too many bytes");
        for s in self.states_mut() {
            match *s {
                State::Nfa(thompson::State::ByteRange {
                    trans: thompson::Transition { start, end, next },
                }) if start == end && start < lt => {
                    *s = State::IbMatcher { matcher: matcher(start), next };
                }
                _ => (),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use regex_automata::Match;
    use regex_syntax::ParserBuilder;

    use crate::{
        matcher::PinyinMatchConfig, pinyin::PinyinNotation,
        regex::nfa::backtrack::BoundedBacktracker, syntax::regex::hir,
    };

    use super::*;

    #[test]
    fn patch_first_byte() {
        let mut nfa = NFA::new("pyss").unwrap();
        dbg!(&nfa);

        nfa.patch_first_byte_to_matcher(
            b'p',
            IbMatcher::builder("p")
                .pinyin(PinyinMatchConfig::notations(
                    PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
                ))
                .build(),
        );
        dbg!(&nfa);

        let re = BoundedBacktracker::new_from_nfa(nfa).unwrap();
        let mut cache = re.create_cache();
        assert_eq!(
            re.try_find(&mut cache, "拼yss").unwrap(),
            Some(Match::must(0, 0..6)),
        );
    }

    #[test]
    fn patch_bytes() {
        let (hir, literals) =
            hir::fold::parse_and_fold_literal_utf8("pyss").unwrap();
        let mut nfa: NFA =
            thompson::Compiler::new().build_from_hir(&hir).unwrap().into();
        nfa.patch_bytes_to_matchers(
            literals.len() as u8,
            literals.len(),
            |b| {
                IbMatcher::builder(literals[b as usize].as_str())
                    .pinyin(PinyinMatchConfig::notations(
                        PinyinNotation::Ascii
                            | PinyinNotation::AsciiFirstLetter,
                    ))
                    .build()
            },
        );
        dbg!(&nfa);
        let re = BoundedBacktracker::new_from_nfa(nfa).unwrap();
        let mut cache = re.create_cache();
        assert_eq!(
            re.try_find(&mut cache, "拼音搜索").unwrap(),
            Some(Match::must(0, 0..12)),
        );
    }

    #[test]
    fn patch_bytes_conflict_gt() {
        let mut parser = ParserBuilder::new().case_insensitive(true).build();
        let hir = parser.parse("δ").unwrap();

        let (mut hirs, literals) =
            hir::fold::fold_literal_utf8(std::iter::once(hir));
        let hir = hirs.pop().unwrap();

        let mut nfa: NFA =
            thompson::Compiler::new().build_from_hir(&hir).unwrap().into();
        dbg!(&nfa);

        nfa.patch_bytes_to_matchers(
            literals.len() as u8,
            literals.len(),
            |b| {
                IbMatcher::builder(literals[b as usize].as_str())
                    .pinyin(PinyinMatchConfig::notations(
                        PinyinNotation::Ascii
                            | PinyinNotation::AsciiFirstLetter,
                    ))
                    .build()
            },
        );
        dbg!(&nfa);

        let re = BoundedBacktracker::new_from_nfa(nfa).unwrap();
        let mut cache = re.create_cache();
        assert_eq!(
            re.try_find(&mut cache, "Δ").unwrap(),
            Some(Match::must(0, 0..2)),
        );
    }

    #[should_panic(expected = "Too many bytes")]
    #[test]
    fn patch_bytes_conflict_lt() {
        let (hir, literals) = hir::fold::parse_and_fold_literal_utf8(
            // r"a([\x00-\x00\u0100\u0200])",
            &format!(r"{}[\u0100\u0200]", "(a)".repeat(129)),
        )
        .unwrap();
        dbg!(&hir, &literals);

        let mut nfa: NFA =
            thompson::Compiler::new().build_from_hir(&hir).unwrap().into();
        dbg!(&nfa);

        nfa.patch_bytes_to_matchers(
            literals.len() as u8,
            literals.len(),
            |b| {
                IbMatcher::builder(literals[b as usize].as_str())
                    .pinyin(PinyinMatchConfig::notations(
                        PinyinNotation::Ascii
                            | PinyinNotation::AsciiFirstLetter,
                    ))
                    .build()
            },
        );
        dbg!(&nfa);

        let re = BoundedBacktracker::new_from_nfa(nfa).unwrap();
        let mut cache = re.create_cache();
        assert_eq!(
            re.try_find(&mut cache, "Δ").unwrap(),
            Some(Match::must(0, 0..2)),
        );
    }

    #[test]
    fn patch_bytes_alt() {
        let (hir, literals) =
            hir::fold::parse_and_fold_literal_utf8("samwise|sam").unwrap();
        dbg!(&hir, &literals);

        let mut nfa: NFA =
            thompson::Compiler::new().build_from_hir(&hir).unwrap().into();
        dbg!(&nfa);

        nfa.patch_bytes_to_matchers(
            literals.len() as u8,
            literals.len(),
            |b| {
                IbMatcher::builder(literals[b as usize].as_str())
                    .pinyin(PinyinMatchConfig::notations(
                        PinyinNotation::Ascii
                            | PinyinNotation::AsciiFirstLetter,
                    ))
                    .build()
            },
        );
        dbg!(&nfa);

        let re = BoundedBacktracker::new_from_nfa(nfa).unwrap();
        let mut cache = re.create_cache();
        assert_eq!(
            re.try_find(&mut cache, "sam").unwrap(),
            Some(Match::must(0, 0..3)),
        );
    }
}

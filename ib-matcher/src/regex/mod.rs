//! ## Design
//! A copy-and-patch NFA.
//!
//! To reduce binary size and maintenance cost, we do not copy the entire `regex_automata` crate, but only the backtrack engine and add a wrapper around `NFA`. The [`NFA`](nfa::NFA) wrapper allows us to inject our own [`State`](nfa::State) variants and copy-and-patch the compiled states.
//!
//! The backtrack engine is forked from [`regex_automata::nfa::thompson::backtrack`](https://docs.rs/regex-automata/0.4.9/regex_automata/nfa/thompson/backtrack/index.html).

#[cfg(not(doctest))]
pub mod backtrack;
mod nfa;
#[cfg(feature = "regex-syntax")]
pub mod syntax;
mod util;

pub use nfa::{State, NFA};

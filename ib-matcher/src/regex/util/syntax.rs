pub use regex_automata::util::syntax::*;

/// - When `regex-unicode` feature is disabled, `unicode` will default to `false` to
///   avoid reporting errors with character classes like `regex` crate does by default.
pub fn config_auto() -> Config {
    #[cfg(feature = "regex-unicode")]
    let c = Config::new();
    #[cfg(not(feature = "regex-unicode"))]
    let c = Config::new().unicode(false);
    c
}

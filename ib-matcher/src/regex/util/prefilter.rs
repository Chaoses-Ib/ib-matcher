pub use regex_automata::util::prefilter::*;

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum PrefilterIb {
    #[cfg(feature = "perf-literal-substring")]
    Byte2OrNonAscii(u8, u8),
}

#[cfg(feature = "perf-literal-substring")]
impl PrefilterIb {
    pub fn byte2_or_non_ascii(b: u8, case_insensitive: bool) -> Self {
        let (a, b) = if case_insensitive {
            // Lowercase letters occur more often
            if b.is_ascii_lowercase() {
                (b, b.to_ascii_uppercase())
            } else {
                (b.to_ascii_lowercase(), b)
            }
        } else {
            (b, b)
        };
        Self::Byte2OrNonAscii(a, b)
    }
}

/*!
## Case folding
> Case folding, i.e. mapping strings to a canonical form for string comparison, typically results in lowercase characters; however, characters in the Cherokee script resolve to uppercase characters. Case folding isn't context-, language-, or locale-sensitive; however, you can specify whether to use mappings for languages like Turkish.

Currently, only simple [case folding](https://www.unicode.org/Public/16.0.0/ucd/CaseFolding.txt) is supported. Simple case folding does not handle some special letter cases that have multiple characters, like `Maße` cannot match `MASSE`.

The API is [`CharCaseExt::to_simple_fold_case()`] and [`StrCaseExt::to_simple_fold_case()`], for example:
```
use ib_unicode::case::StrCaseExt;

assert_eq!("βίος".to_simple_fold_case(), "βίοσ");
assert_eq!("Βίοσ".to_simple_fold_case(), "βίοσ");
assert_eq!("ΒΊΟΣ".to_simple_fold_case(), "βίοσ");
```

- Unicode version: 16.0.0.
- Performance: The default implementation uses the same algorithm as the `unicase` crate, which is compact but a bit slow, especially on miss paths. You can enable the `perf-case-fold` feature to use a faster algorithm.

Simple case folding is also used by the [`regex`](https://docs.rs/regex/) crate.

## Mono lowercase
The "mono lowercase" mentioned in this module refers to the single-char lowercase mapping of a Unicode character. This is different from Unicode's [simple case folding](#case-folding) in that it always results in lowercase characters, and does not normalize different lower cases of a character to the same one (e.g. `σ` and `ς` are kept).

<!-- except that some full/special case foldings are also added but only kept the first character (currently only `İ`). -->

For example:
```
use ib_unicode::case::StrCaseExt;

assert_eq!("βίος".to_mono_lowercase(), "βίος");
assert_eq!("Βίοσ".to_mono_lowercase(), "βίοσ");
assert_eq!("ΒΊΟΣ".to_mono_lowercase(), "βίοσ");
```

- Unicode version: 16.0.0.
- Compared to [`char::to_lowercase()`]/[`str::to_lowercase()`] in `std`: the same, except that `İ` is mapped to `i` instead of `i\u{307}`.
  - `Σ` always maps to `σ` instead of conditionally `ς`, unlike in `str::to_lowercase()`. This may be changed if the need arises.
  - [`to_mono_lowercase()`](CharCaseExt::to_mono_lowercase) is also much faster if `perf-case-map` feature is enabled.
- Compared to simple case folding: Besides normalization, the covered characters are basically the same, except that there is no `İ` in simple case folding but the following ones:
  - ΐ, ΐ
  - ΰ, ΰ
  - ﬅ, ﬆ
*/

use crate::Sealed;

#[cfg(feature = "case-fold")]
mod fold;
#[cfg(feature = "perf-case-map")]
mod map;

pub trait CharCaseExt: Sealed {
    /// The only multi-char lowercase mapping is 'İ' -> "i\u{307}", we just ignore the '\u{307}'.
    ///
    /// See [mono lowercase](super::case#mono-lowercase) for details.
    fn to_mono_lowercase(self) -> char;

    /// A convenient method for feature-gated case folding.
    /// If `case-fold` feature is enabled, it uses simple case folding; otherwise it uses `to_ascii_lowercase()`.
    fn to_simple_or_ascii_fold_case(self) -> char;

    /// See [case folding](super::case#case-folding) for details.
    #[cfg(feature = "case-fold")]
    fn to_simple_fold_case(self) -> char;

    /// See [case folding](super::case#case-folding) for details.
    #[cfg(feature = "bench")]
    fn to_simple_fold_case_unicase(self) -> char;

    /// See [case folding](super::case#case-folding) for details.
    #[cfg(feature = "bench")]
    fn to_simple_fold_case_map(self) -> char;
}

impl CharCaseExt for char {
    fn to_mono_lowercase(self) -> char {
        #[cfg(not(feature = "perf-case-map"))]
        return self.to_lowercase().next().unwrap();

        // Optimize away the binary search
        // Reduce total match time by ~37%
        #[cfg(feature = "perf-case-map")]
        map::to_mono_lowercase(self)
    }

    fn to_simple_or_ascii_fold_case(self) -> char {
        #[cfg(not(feature = "case-fold"))]
        return self.to_ascii_lowercase();
        #[cfg(feature = "case-fold")]
        self.to_simple_fold_case()
    }

    #[cfg(feature = "case-fold")]
    fn to_simple_fold_case(self) -> char {
        #[cfg(not(feature = "perf-case-fold"))]
        return fold::unicase::fold(self);
        #[cfg(feature = "perf-case-fold")]
        fold::map::fold(self)
    }

    #[cfg(feature = "bench")]
    fn to_simple_fold_case_unicase(self) -> char {
        fold::unicase::fold(self)
    }

    #[cfg(feature = "bench")]
    fn to_simple_fold_case_map(self) -> char {
        fold::map::fold(self)
    }
}

pub trait StrCaseExt: Sealed {
    /// See [mono lowercase](super::case#mono-lowercase) for details.
    fn to_mono_lowercase(&self) -> String;

    /// A convenient method for feature-gated case folding.
    /// If `case-fold` feature is enabled, it uses simple case folding; otherwise it uses `to_ascii_lowercase()`.
    fn to_simple_or_ascii_fold_case(&self) -> String;

    /// See [case folding](super::case#case-folding) for details.
    #[cfg(feature = "case-fold")]
    fn to_simple_fold_case(&self) -> String;
}

impl StrCaseExt for str {
    fn to_mono_lowercase(&self) -> String {
        self.chars().map(|c| c.to_mono_lowercase()).collect()
    }

    fn to_simple_or_ascii_fold_case(&self) -> String {
        self.chars()
            .map(|c| c.to_simple_or_ascii_fold_case())
            .collect()
    }

    #[cfg(feature = "case-fold")]
    fn to_simple_fold_case(&self) -> String {
        self.chars().map(|c| c.to_simple_fold_case()).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn mono_set() -> HashSet<char> {
        let mut chars = HashSet::new();
        for c in 'A'..='Z' {
            chars.insert(c);
            chars.insert(c.to_ascii_lowercase());
        }
        for (c, map) in map::tests::LOWERCASE_TABLE {
            chars.insert(*c);
            chars.insert(char::from_u32(*map).unwrap_or('i'));
        }
        chars
    }

    #[test]
    fn mono() {
        let mono = mono_set();
        println!("{} chars", mono.len());
        println!("{} upper chars", 26 + map::tests::LOWERCASE_TABLE.len());
    }
}

/// ucd-generate case-folding-simple ucd-16.0.0 --chars --all-pairs > case-folding-simple-chars-all-pairs.rs
#[cfg(all(not(feature = "doc"), feature = "_test_data"))]
mod tests_data {
    use std::collections::HashSet;

    include!("../../data/case-folding-simple-chars-all-pairs.rs");

    fn regex_set() -> HashSet<char> {
        let mut chars = HashSet::new();
        for (c, maps) in CASE_FOLDING_SIMPLE {
            chars.insert(*c);
            for c in maps.iter() {
                chars.insert(*c);
            }
        }
        chars
    }

    #[test]
    fn regex() {
        let regex = regex_set();
        println!("{} chars", regex.len());
    }

    #[test]
    fn mono_sub_regex() {
        let regex = regex_set();

        let mut chars = HashSet::new();
        for (c, map) in map::tests::LOWERCASE_TABLE {
            if !regex.contains(c) {
                chars.insert(*c);
            }
            let map = char::from_u32(*map).unwrap_or('i');
            if !regex.contains(&map) {
                chars.insert(map);
            }
        }
        println!("{} chars", chars.len());
        println!("{:?}", chars);
    }

    #[test]
    fn regex_sub_mono() {
        let mono = mono_set();

        let mut chars = HashSet::new();
        let mut multicase = HashSet::new();
        for (c, maps) in CASE_FOLDING_SIMPLE {
            let set = if maps.len() > 1 {
                &mut multicase
            } else {
                &mut chars
            };
            if !mono.contains(c) {
                set.insert(*c);
            }
            for c in maps.iter() {
                if !mono.contains(c) {
                    set.insert(*c);
                }
            }
        }
        println!("{} chars", chars.len());
        println!("{} multicase chars", multicase.len());
        println!("{:?}", chars);
        println!("{:?}", multicase);
    }
}

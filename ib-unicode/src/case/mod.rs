/*!
## Mono lowercase
The "mono lowercase" mentioned in this module refers to the single-char lowercase mapping of a Unicode character. This is the same as Unicode's simple [case folding](https://www.unicode.org/Public/16.0.0/ucd/CaseFolding.txt), except that some full/special case foldings are also added but only kept the first character (currently only `İ`).

- Unicode version: 16.0.0.
- Compared to [`char::to_lowercase()`]/[`str::to_lowercase()`] in `std`: the same, except that `İ` is mapped to `i` instead of `i\u{307}`.
  - [`to_mono_lowercase()`](CharToMonoLowercase::to_mono_lowercase) is also much faster if `perf-case-map` feature is enabled.
- Compared to the [`regex`](https://docs.rs/regex/) crate: the same, except that `regex` does not add `İ` but the following full case foldings (with only the first char):
  - ΐ, ΐ
  - ΰ, ΰ
  - ﬅ, ﬆ
*/
#[cfg(feature = "perf-case-map")]
mod map;

pub trait CharToMonoLowercase {
    /// The only multi-char lowercase mapping is 'İ' -> "i\u{307}", we just ignore the '\u{307}'.
    ///
    /// See [mono lowercase](super::case) for details.
    fn to_mono_lowercase(self) -> char;
}

impl CharToMonoLowercase for char {
    fn to_mono_lowercase(self) -> char {
        #[cfg(not(feature = "perf-case-map"))]
        return self.to_lowercase().next().unwrap();

        // Optimize away the binary search
        // Reduce total match time by ~37%
        #[cfg(feature = "perf-case-map")]
        map::to_mono_lowercase(self)
    }
}

pub trait StrToMonoLowercase {
    /// See [mono lowercase](super::case) for details.
    fn to_mono_lowercase(&self) -> String;
}

impl StrToMonoLowercase for str {
    fn to_mono_lowercase(&self) -> String {
        self.chars().map(|c| c.to_mono_lowercase()).collect()
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

    #[cfg(feature = "_test_regex")]
    include!("../../data/case_folding_simple.rs");

    #[cfg(feature = "_test_regex")]
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

    #[cfg(feature = "_test_regex")]
    #[test]
    fn regex() {
        let regex = regex_set();
        println!("{} chars", regex.len());
    }

    #[cfg(feature = "_test_regex")]
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

    #[cfg(feature = "_test_regex")]
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

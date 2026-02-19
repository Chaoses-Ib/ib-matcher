use logos::Logos;

use crate::HepburnRomanizer;

#[derive(Logos, Clone, Copy, Debug, PartialEq)]
#[logos(utf8 = false)]
enum RomajiToken {
    /// All kanas except `n'*`, since they are actually two kanas.
    /// (`"(?![っッ])[^"]{2,}"`)
    ///
    /// `n'a|n'e|n'i|n'o|n'u|n'ya|n'yo|n'yu` -> `'`
    #[regex(
        "(?x)a|ba|bba|bbe|bbi|bbo|bbu|bbya|bbyo|bbyu|be|bi|bo|bu|bya|byo|byu|cha|che|chi|cho|chu|da|dda|dde|ddo|de|di|do
        |e|fa|fe|ffa|ffe|ffi|ffo|ffu|fi|fo|fu|ga|ge|gga|gge|ggi|ggo|ggu|ggya|ggyo|ggyu|gi|go|gu|gya|gyo|gyu
        |ha|he|hha|hhe|hhi|hho|hhya|hhyo|hhyu|hi|ho|hya|hyo|hyu|i|ja|ji|jja|jji|jjo|jju|jjya|jjyo|jjyu|jo|ju
        |ka|ke|ki|kka|kke|kki|kko|kku|kkya|kkyo|kkyu|ko|ku|kya|kyo|kyu|ma|me|mi|mo|mu|mya|myo|myu
        |n|na|ne|ni|no|nu|nya|nyo|nyu
        |o|pa|pe|pi|po|ppa|ppe|ppi|ppo|ppu|ppya|ppyo|ppyu|pu|pya|pyo|pyu|ra|re|ri|ro|rra|rre|rri|rro|rru|rrya|rryo|rryu|ru|rya|ryo|ryu
        |sa|se|sha|shi|sho|shu|so|ssa|sse|ssha|sshi|ssho|sshu|sso|ssu|su|ta
        |tcha|tchi|tcho|tchu
        |te|to|tsu|tta|tte|tto|ttsu|u|va|ve|vi|vo|vu|vva|vve|vvi|vvo|vvu|wa|we|wi|wo|ya|yo|yu|yya|yyo|yyu|za|ze|zo|zu|zza|zzo|zzu"
    )]
    Kana,

    #[token("'")]
    Apostrophe,

    #[regex("[^a-z']")]
    Other,
}

impl HepburnRomanizer {
    pub const POSSIBLE_PREFIX: char = 'n';

    pub const APOSTROPHE: char = '\'';
    pub const APOSTROPHE_STR: &str = "'";

    #[inline]
    fn is_romaji_n_suffix(next: u8) -> bool {
        matches!(next, b'a' | b'e' | b'i' | b'o' | b'u' | b'y')
    }

    pub fn need_apostrophe_c<'s>(last_char: char, romaji: &'s str) -> bool {
        let b = romaji.as_bytes()[0];
        last_char == Self::POSSIBLE_PREFIX && Self::is_romaji_n_suffix(b)
    }

    pub fn need_apostrophe<'s>(last_romaji: &str, romaji: &'s str) -> bool {
        let b = romaji.as_bytes()[0];
        last_romaji.ends_with(Self::POSSIBLE_PREFIX) && Self::is_romaji_n_suffix(b)
    }

    /// Test if a `n` in a *legal* romaji string is a kana "ん(n)",
    /// not in the middle of kanas "なねにのぬ にゃ にょ にゅ (n*)".
    pub fn is_romaji_n_boundary(s: &str, index: usize) -> bool {
        debug_assert!(s.is_ascii());
        let s = s.as_bytes();
        debug_assert_eq!(s[index], b'n');
        if let Some(&next) = s.get(index + 1) {
            // Possible positive values: ' bcdfghjkmprstvwz
            // Possible negative values: aeiou y
            // nyya んっや?
            !Self::is_romaji_n_suffix(next)
        } else {
            true
        }
    }

    /// Test if the index of a *legal* romaji string is a kana boundary,
    /// i.e. not in the middle of a kana.
    ///
    /// ### Performance
    /// ~1 ns per byte, i.e. 1 GB/s.
    ///
    /// It is possible to avoid scanning from the beginning. But for the current
    /// usage, most input (kanji and word kanas) is short, scanning from the beginning
    /// is friendlier to cache and maybe faster.
    ///
    /// `#[inline]` is needed for mitigating rustc's optimization bug related to generics,
    /// although maybe not fully.
    #[inline]
    pub fn is_romaji_kana_boundary(s: impl AsRef<[u8]>, index: usize) -> bool {
        use std::cmp::Ordering;

        let s = s.as_ref();
        debug_assert!(index < s.len());

        /*
        let mut lex = RomajiToken::lexer(unsafe { s.get_unchecked(..index) });
        while let Some(r) = lex.next() {
            if r.is_err() {
                return false;
            }
        }
        {}
        true
        */

        // Alternatively, is_romaji_n_boundary() can also be used
        let mut lex = RomajiToken::lexer(s);
        loop {
            // Unmatched char will still move span()
            /*
            let r = lex.next();
            debug_assert!(r.is_none_or(|r| r.is_ok()));
            match lex.span().start.cmp(&index) {
                Ordering::Less => (),
                Ordering::Equal => return r.is_none_or(|r| r.is_ok()),
                Ordering::Greater => return false,
            }
            */
            // ~10% faster
            match lex.next() {
                Some(Ok(_)) => match lex.span().start.cmp(&index) {
                    Ordering::Less => (),
                    Ordering::Equal => return true,
                    Ordering::Greater => return false,
                },
                Some(Err(_)) => return false,
                None => return false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeSet;

    use crate::data::kana::HEPBURN_ROMAJIS;

    #[test]
    fn hepburn_prefix() {
        let mut romaji_set = BTreeSet::new();
        for &romaji in HEPBURN_ROMAJIS {
            romaji_set.insert(romaji);
        }

        println!("Sorted and deduplicated romaji:");
        dbg!(romaji_set.len());
        for romaji in &romaji_set {
            println!("{}", romaji);
        }

        // Ensure no str is prefix of other strs in romaji_set
        let romaji_vec: Vec<&str> = romaji_set.iter().copied().collect();
        for i in 0..romaji_vec.len() {
            for j in (i + 1)..romaji_vec.len() {
                if romaji_vec[i] == HepburnRomanizer::POSSIBLE_PREFIX.to_string() {
                    continue;
                }
                assert!(
                    !romaji_vec[j].starts_with(romaji_vec[i]),
                    "Romaji '{}' is a prefix of '{}'",
                    romaji_vec[i],
                    romaji_vec[j]
                );
            }
        }
    }

    #[test]
    fn is_romaji_n_boundary() {
        // Test cases where 'n' should be treated as a boundary
        let boundary_cases = vec![
            "n", "n'a", "n'e", "n'i", "n'o", "n'u", "nja", "nfu", "n!", "n1", "nba",
        ];
        for case in boundary_cases {
            assert!(
                HepburnRomanizer::is_romaji_n_boundary(case, 0),
                "Failed for boundary case: {case}",
            );
        }

        // Test cases where 'n' should NOT be treated as a boundary
        let non_boundary_cases = vec!["na", "ne", "ni", "no", "nu", "nya", "nyo", "nyu"];
        for case in non_boundary_cases {
            assert!(
                !HepburnRomanizer::is_romaji_n_boundary(case, 0),
                "Failed for non-boundary case: {case}",
            );
        }

        // Test cases with 'n' in the middle of string
        let middle_cases = vec![
            ("kana", 2),
            ("kane", 2),
            ("kani", 2),
            ("kano", 2),
            ("kanu", 2),
            ("kany", 2),
            ("kanya", 1),
            ("kanyo", 1),
            ("kanyu", 1),
        ];

        for (string, index) in middle_cases {
            let s = string.as_bytes();
            if s[index] == b'n' {
                if matches!(s[index + 1], b'a' | b'e' | b'i' | b'o' | b'u' | b'y') {
                    assert!(
                        !HepburnRomanizer::is_romaji_n_boundary(string, index),
                        "Failed for middle case: {string} at index {index}"
                    );
                } else {
                    assert!(
                        HepburnRomanizer::is_romaji_n_boundary(string, index),
                        "Failed for middle case: {string} at index {index}"
                    );
                }
            }
        }
    }

    #[test]
    fn is_romaji_kana_boundary() {
        for (s, i, r) in vec![
            // ("ab", 2, false),
            // ka nya
            ("kanya", 0, true),
            ("kanya", 1, false),
            ("kanya", 2, true),
            ("kanya", 3, false),
            ("kanya", 4, false),
            // ("kanya", 5, true),
            // ka n ni
            ("kanni", 3, true),
            ("kanni", 4, false),
            // ("kanni", 5, true),
            // ka n n i
            ("kann'i", 4, true),
            ("kann'i", 5, true),
            // ("kann'i", 6, true),
        ] {
            assert_eq!(
                HepburnRomanizer::is_romaji_kana_boundary(s, i),
                r,
                "{s}, {i}"
            );
        }
    }
}

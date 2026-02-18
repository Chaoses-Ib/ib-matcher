use crate::HepburnRomanizer;

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
}

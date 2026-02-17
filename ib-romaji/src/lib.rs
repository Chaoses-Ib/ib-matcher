//! A fast Japanese romanizer.
//!
//! ## Usage
//! ```rust
//! use ib_romaji::HepburnRomanizer;
//!
//! let romanizer = HepburnRomanizer::default();
//!
//! let mut romajis = Vec::new();
//! romanizer.romanize_and_try_for_each("日本語", |len, romaji| {
//!     romajis.push((len, romaji));
//!     None::<()>
//! });
//! assert_eq!(romajis, vec![(9, "nippongo"), (3, "a"), (3, "aki"), (3, "bi"), (3, "chi"), (3, "he"), (3, "hi"), (3, "iru"), (3, "jitsu"), (3, "ka"), (3, "kou"), (3, "ku"), (3, "kusa"), (3, "nchi"), (3, "ni"), (3, "nichi"), (3, "nitsu"), (3, "su"), (3, "tachi")]);
//!
//! assert_eq!(romanizer.romanize_vec("日本語"), vec![(9, "nippongo"), (3, "a"), (3, "aki"), (3, "bi"), (3, "chi"), (3, "he"), (3, "hi"), (3, "iru"), (3, "jitsu"), (3, "ka"), (3, "kou"), (3, "ku"), (3, "kusa"), (3, "nchi"), (3, "ni"), (3, "nichi"), (3, "nitsu"), (3, "su"), (3, "tachi")]);
//! ```
//!
//! ## Binary size
//! The dictionary will take ~4.8 MiB (5.5 MiB without compression) in the binary at the moment.
//!
//! ## Design
//! `&[&str]` will cause each str to occupy 16 extra bytes to store the pointer and length. While CStr only needs 1 byte for each str.
//! - For words, this can save 3.14 MiB (actually 3.54 MiB).
//!   - Source file: 2.98 MiB -> `\0`+`\`: 2.80 MiB, `\n`: 2.54 MiB
//!   - `build()` time: `split()`/memchr +10%
//! - And this way the str can also be compressed and then streamly decompressed.
//!
//! ## Features
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "doc", doc = document_features::document_features!())]
use bon::bon;
use daachorse::{CharwiseDoubleArrayAhoCorasick, CharwiseDoubleArrayAhoCorasickBuilder, MatchKind};

use ib_unicode::str::RoundCharBoundaryExt;

#[cfg(feature = "cache")]
pub mod cache;
pub mod data;
mod input;
pub mod kanji;

pub use input::Input;

/// [Hepburn romanization](https://en.wikipedia.org/wiki/Hepburn_romanization)
#[derive(Clone)]
pub struct HepburnRomanizer {
    // ac: AhoCorasick,
    ac: CharwiseDoubleArrayAhoCorasick<u32>,
    kanji: bool,
}

#[bon]
impl HepburnRomanizer {
    /// [`HepburnRomanizer::default()`]
    #[builder(builder_type = HepburnRomanizerBuilder, state_mod(vis = "pub(crate)"))]
    pub fn new(
        #[builder(default = false, getter(vis = "pub(crate)"))] kana: bool,
        #[builder(default = false, getter(vis = "pub(crate)"))] kanji: bool,
        #[builder(default = false, getter(vis = "pub(crate)"))] word: bool,
    ) -> Self {
        // // let start = UnsafeCell::new(0);
        // let mut start = 0;
        // let words = memchr::memchr_iter(b'\n', data::WORDS.as_bytes()).map(|end| {
        //     // let start = start.get();
        //     // let word = unsafe { str::from_raw_parts(data::WORDS.as_ptr().add(start), end - start) };
        //     let word = unsafe { data::WORDS.get_unchecked(start..end) };
        //     start = end + 1;
        //     word
        // });
        // // chain() will make the iterator significantly slower
        // // .chain(iter::once(unsafe {
        // //     data::WORDS.get_unchecked(*start.get()..)
        // // }));

        // memchr is as fast as std, but harder to work with
        #[cfg(not(feature = "compress-words"))]
        let words = data::WORDS.split('\n');
        #[cfg(feature = "compress-words")]
        let words = include_bytes_zstd::include_bytes_zstd!("src/data/words.in.txt", 22);
        #[cfg(feature = "compress-words")]
        let words = words
            .split(|&b| b == b'\n')
            .map(|b| unsafe { str::from_utf8_unchecked(b) });

        // let mut ac = AhoCorasick::builder();
        // ac.start_kind(StartKind::Anchored)
        //     .match_kind(MatchKind::LeftmostLongest);
        // let ac = match (kana, word) {
        //     (true, true) => ac.build(data::kana::HEPBURN_KANAS.iter().cloned().chain(words)),
        //     (true, false) => ac.build(data::kana::HEPBURN_KANAS),
        //     (false, true) => ac.build(words),
        //     (false, false) => ac.build::<_, &str>([]),
        // }
        // .unwrap();

        let ac =
            CharwiseDoubleArrayAhoCorasickBuilder::new().match_kind(MatchKind::LeftmostLongest);
        let ac = match (kana, word) {
            (true, true) => ac.build(data::kana::HEPBURN_KANAS.iter().cloned().chain(words)),
            (true, false) => ac.build(data::kana::HEPBURN_KANAS),
            (false, true) => ac.build(words),
            (false, false) => ac.build([] as [&str; 0]),
        }
        .unwrap();

        Self { ac, kanji }
    }

    /// Romanize the first kana in the string, and return the length of the kana and the romaji.
    ///
    /// ## Example
    /// ```
    /// use ib_romaji::HepburnRomanizer;
    ///
    /// assert_eq!(HepburnRomanizer::builder().kana(true).build().romanize_kana("あ"), Some((3, "a")));
    /// ```
    /// TODO: Iter
    pub fn romanize_kana<S: ?Sized + AsRef<str>>(&self, s: &S) -> Option<(usize, &'static str)> {
        let s = s.as_ref();
        let s = &s[..s.floor_char_boundary_ib(data::kana::KANA_MAX_LEN)];
        // let m = self.ac.find(Input::new(s).anchored(Anchored::Yes))?;
        // let pattern = m.pattern().as_usize();
        let m = self
            .ac
            .leftmost_find_iter(s)
            .next()
            .filter(|m| m.start() == 0)?;
        let pattern = m.value() as usize;
        let len = m.end() - m.start();
        data::kana::HEPBURN_ROMAJIS
            .get(pattern)
            .map(|&romaji| (len, romaji))
    }

    /// Romanize kanas from the beginning of the string until a non-kana character, and return the length of the kanas and the romajis.
    pub fn romanize_kana_str<S: ?Sized + AsRef<str>>(&self, s: &S) -> Option<(usize, String)> {
        let s = s.as_ref();
        let mut len = 0;
        let mut buf = String::new();
        while let Some((l, romaji)) = self.romanize_kana(&s[len..]).or_else(|| {
            if s[len..].starts_with("、") {
                Some((3, "、"))
            } else {
                None
            }
        }) {
            len += l;
            buf.push_str(romaji);
            if len >= s.len() {
                return Some((len, buf));
            }
        }
        if len == 0 { None } else { Some((len, buf)) }
    }

    /// Romanize kana text to romajis. Returns `None` if there is any non-kana character in the string.
    pub fn romanize_kana_str_all<S: ?Sized + AsRef<str>>(&self, s: &S) -> Option<String> {
        let s = s.as_ref();
        match self.romanize_kana_str(s) {
            Some((len, buf)) if len == s.len() => Some(buf),
            _ => None,
        }
    }

    /// Romanize the first word in the string, and call `f` for each possible romanization.
    ///
    /// `f` can return `Some(_)` to stop the iteration, or `None` to continue.
    ///
    /// ## Example
    /// ```
    /// use ib_romaji::HepburnRomanizer;
    ///
    /// let mut romajis = Vec::new();
    /// HepburnRomanizer::default().romanize_and_try_for_each("日本語", |len, romaji| {
    ///     romajis.push((len, romaji));
    ///     None::<()>
    /// });
    /// assert_eq!(romajis, vec![(9, "nippongo"), (3, "a"), (3, "aki"), (3, "bi"), (3, "chi"), (3, "he"), (3, "hi"), (3, "iru"), (3, "jitsu"), (3, "ka"), (3, "kou"), (3, "ku"), (3, "kusa"), (3, "nchi"), (3, "ni"), (3, "nichi"), (3, "nitsu"), (3, "su"), (3, "tachi")]);
    /// ```
    ///
    /// ## See also
    /// [`romanize_vec()`](Self::romanize_vec) for a version that returns a `Vec` of all possible romanizations.
    pub fn romanize_and_try_for_each<'h, S: Into<Input<'h>>, T>(
        &self,
        input: S,
        mut f: impl FnMut(usize, &'static str) -> Option<T>,
    ) -> Option<T> {
        let input = input.into();
        let s = input.as_ref();
        let s = &s[..s.floor_char_boundary_ib(data::WORD_MAX_LEN)];

        // self.ac.find(Input::new(s).anchored(Anchored::Yes))
        if let Some(m) = self
            .ac
            .leftmost_find_iter(s)
            .next()
            .filter(|m| m.start() == 0)
        {
            // let pattern = m.pattern().as_usize();
            let pattern = m.value() as usize;
            let len = m.end() - m.start();
            if pattern < data::kana::HEPBURN_ROMAJIS.len() {
                let romaji = data::kana::HEPBURN_ROMAJIS[pattern];
                if let Some(result) = f(len, romaji) {
                    return Some(result);
                }
            } else if pattern < data::kana::HEPBURN_ROMAJIS.len() + data::WORD_ROMAJIS.len() {
                // TODO: Binary search
                for romaji in data::WORD_ROMAJIS[pattern - data::kana::HEPBURN_ROMAJIS.len()] {
                    if let Some(result) = f(len, romaji) {
                        return Some(result);
                    }
                }
            }
        }

        if self.kanji {
            if let Some(result) = self.romanize_kanji_and_try_for_each(input, f) {
                return Some(result);
            }
        }

        None
    }

    /// Romanize the first word in the string, and return a `Vec` for all possible romanization.
    ///
    /// ## Example
    /// ```
    /// use ib_romaji::HepburnRomanizer;
    ///
    /// assert_eq!(HepburnRomanizer::default().romanize_vec("日本語"), vec![(9, "nippongo"), (3, "a"), (3, "aki"), (3, "bi"), (3, "chi"), (3, "he"), (3, "hi"), (3, "iru"), (3, "jitsu"), (3, "ka"), (3, "kou"), (3, "ku"), (3, "kusa"), (3, "nchi"), (3, "ni"), (3, "nichi"), (3, "nitsu"), (3, "su"), (3, "tachi")]);
    /// ```
    pub fn romanize_vec<'h, S: Into<Input<'h>>>(&self, s: S) -> Vec<(usize, &'static str)> {
        let mut results = Vec::new();
        self.romanize_and_try_for_each(s, |len, romaji| {
            results.push((len, romaji));
            None::<()>
        });
        results
    }

    /// Check if the string can be fully romanized.
    ///
    /// This function can be used to test if the string is a possible Japanese text or not.
    pub fn is_romanizable<'h, S: Into<Input<'h>>>(&self, s: S) -> bool {
        let s = s.into();
        if s.is_empty() {
            return true;
        }
        self.romanize_and_try_for_each(s, |len, _| {
            self.is_romanizable(Input::new(s.haystack(), s.start() + len))
                .then_some(())
        })
        .is_some()
    }

    /// Check if the string can be fully romanized to the given romaji.
    pub fn is_romanizable_to<'h, S: Into<Input<'h>>>(
        &self,
        s: S,
        romaji: &(impl ?Sized + AsRef<str>),
    ) -> bool {
        let s = s.into();
        let romaji = romaji.as_ref();
        if s.is_empty() {
            return romaji.is_empty();
        }
        self.romanize_and_try_for_each(s, |len, word_romaji| {
            self.is_romanizable_to(
                Input::new(s.haystack(), s.start() + len),
                romaji.strip_prefix(word_romaji)?,
            )
            .then_some(())
        })
        .is_some()
    }
}

impl Default for HepburnRomanizer {
    fn default() -> Self {
        Self::builder().kana(true).kanji(true).word(true).build()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use indexmap::IndexSet;

    use super::*;

    #[test]
    fn min_len() {
        let min_len = data::kana::HEPBURN_KANAS
            .iter()
            .inspect(|kana| {
                if kana.len() == data::kana::KANA_MIN_LEN {
                    println!("{}", kana);
                }
            })
            .map(|s| s.len())
            .min()
            .unwrap();
        assert_eq!(data::kana::KANA_MIN_LEN, min_len);

        assert!(data::MIN_LEN <= data::kana::KANA_MIN_LEN);
        assert!(data::MIN_LEN <= data::KANJI_MIN_LEN);
    }

    #[test]
    fn kana_max_len() {
        let max_len = data::kana::HEPBURN_KANAS
            .iter()
            .inspect(|kana| {
                if kana.len() == data::kana::KANA_MAX_LEN {
                    println!("{}", kana);
                }
            })
            .map(|s| s.len())
            .max()
            .unwrap();
        assert_eq!(data::kana::KANA_MAX_LEN, max_len);

        let max_len = data::kana::HEPBURN_ROMAJIS
            .iter()
            .inspect(|romaji| {
                if romaji.len() == data::kana::KANA_ROMAJI_MAX_LEN {
                    println!("{}", romaji);
                }
            })
            .map(|s| s.len())
            .max()
            .unwrap();
        assert_eq!(data::kana::KANA_ROMAJI_MAX_LEN, max_len);
    }

    #[test]
    fn kana() {
        let data = HepburnRomanizer::builder().kana(true).build();
        assert_eq!(data.romanize_kana("は"), Some((3, "ha")));
        assert_eq!(data.romanize_kana("ハハハ"), Some((3, "ha")));
        assert_eq!(data.romanize_kana("ジョジョ"), Some((6, "jo")));
        assert_eq!(data.romanize_kana("って"), Some((6, "tte")));
        assert_eq!(data.romanize_kana("日は"), None);
    }

    #[test]
    fn kana_str() {
        let data = HepburnRomanizer::builder().kana(true).build();
        assert_eq!(data.romanize_kana_str("は"), Some((3, "ha".into())));
        assert_eq!(data.romanize_kana_str("ハハハ"), Some((9, "hahaha".into())));
        assert_eq!(
            data.romanize_kana_str("ジョジョ"),
            Some((12, "jojo".into()))
        );
        assert_eq!(data.romanize_kana_str("って"), Some((6, "tte".into())));
        assert_eq!(data.romanize_kana_str("日は"), None);
    }

    #[test]
    fn is_romanizable_to() {
        let data = HepburnRomanizer::builder().kana(true).kanji(true).build();
        assert!(data.is_romanizable_to("は", "ha"));
        assert!(data.is_romanizable_to("ハハハ", "hahaha"));
        assert!(data.is_romanizable_to("ジョジョ", "jojo"));
        assert!(data.is_romanizable_to("って", "tte"));
        assert!(data.is_romanizable_to("日は", "hiha"));
        assert!(data.is_romanizable_to("日は", "kusaha"));
        assert!(!data.is_romanizable_to("今日", "kyou"));
        assert!(data.is_romanizable_to("今日", "imakusa"));
    }

    #[ignore]
    #[test]
    fn codegen_kanji() {
        let romanizer = HepburnRomanizer::builder().kana(true).build();

        let mut dup_count = 0;
        let mut romaji_max_len = 0;

        let kanjidic = fs::read_to_string("data/kanjidic.csv").unwrap();
        let mut out_kanjis = fs::File::create("src/data/kanjis.rs").unwrap();
        writeln!(out_kanjis, "match kanji {{").unwrap();
        let mut range = 0;
        for (_i, line) in kanjidic.lines().enumerate() {
            let (kanji, kanas) = match line.split_once('\t') {
                Some(v) => v,
                None => continue,
            };
            if matches!(kanji, kanji::NOMA_STR) {
                continue;
            }

            write!(out_kanjis, "'{kanji}'=>").unwrap();

            let kanas_count = kanas.split('\t').count();
            let mut kanas_set: IndexSet<String> = kanas
                .split('\t')
                .map(|kana| match romanizer.romanize_kana_str_all(kana) {
                    Some(romaji) => format!("\"{}\"", romaji),
                    None => {
                        println!("Failed to romanize kana: {kana}");
                        kana.into()
                    }
                })
                .collect();
            kanas_set.sort_unstable();
            if kanas_set.len() != kanas_count {
                // println!("Duplicated romajis: {kanji}\t{kanas}");
                dup_count += 1;
            }

            assert!(
                data::KANJI_LEN.contains(&kanji.len()),
                "{kanji} {}",
                kanji.len()
            );
            {
                let max_len = kanas_set.iter().map(|s| s.len()).max().unwrap();
                if max_len > romaji_max_len {
                    romaji_max_len = max_len;
                }
                if max_len == data::KANJI_ROMAJI_MAX_LEN {
                    println!("Max len romaji: {kanji} {kanas_set:?}");
                }
            }

            write!(
                out_kanjis,
                "&[{}],",
                kanas_set.into_iter().collect::<Vec<_>>().join(",")
            )
            .unwrap();

            // (i + 1) % 8 == 0
            // Natural align
            let c = kanji.chars().next().unwrap() as u32;
            if c / 10 != range {
                range = c / 10;
                out_kanjis.write_all(b"\n").unwrap();
            }
        }
        write!(out_kanjis, "_ => &[]\n}}").unwrap();

        println!("Kanjis with duplicated romajis: {dup_count}");
        println!("Romaji max len: {romaji_max_len}");
        assert_eq!(romaji_max_len, data::KANJI_ROMAJI_MAX_LEN);
    }

    /// `codegen_kanji()` should be run first.
    ///
    /// `cargo test --package ib-romaji --lib -r -- tests::codegen_word --exact --no-capture --ignored > data/word.txt`
    #[ignore]
    #[test]
    fn codegen_word() {
        let romanizer = HepburnRomanizer::builder().kana(true).build();
        let kanji_romanizer = HepburnRomanizer::builder().kana(true).kanji(true).build();

        let mut dup_count = 0;
        let mut romanizable_count = 0;
        let mut partial_romanizable_count = 0;
        let mut diff_romanizable_count = 0;
        let mut unromanizable_count = 0;
        let mut max_len = 0;
        let mut romaji_max_len = 0;

        let jmdict = fs::read_to_string("data/jmdict.csv").unwrap();
        let mut out_words = fs::File::create("src/data/words.in.txt").unwrap();
        let mut out_kanas = fs::File::create("src/data/word_kanas.rs").unwrap();
        // writeln!(out_words, "&[").unwrap();
        // writeln!(out_words, "\"").unwrap();
        // let end = jmdict.lines().count() - 1;
        writeln!(out_kanas, "&[").unwrap();
        // let mut c = 0;
        let mut range = 0;
        let mut range_c = 0;
        let mut range_2 = 0;
        for (i, line) in jmdict.lines().enumerate() {
            let (word, kanas) = match line.split_once('\t') {
                Some(v) => v,
                None => continue,
            };

            let kanas_count = kanas.split('\t').count();
            let kanas_set: IndexSet<String> = kanas
                .split('\t')
                .map(|kana| match romanizer.romanize_kana_str_all(kana) {
                    // format!("\"{}\"", romaji)
                    Some(romaji) => romaji,
                    None => {
                        println!("Failed to romanize kana: {kana}");
                        kana.into()
                    }
                })
                .collect();
            if kanas_set.len() != kanas_count {
                // println!("Duplicated romajis: {kanji}\t{kanas}");
                dup_count += 1;
            }

            // Filter out ordinary words
            // Source file: 2.52+3.59=6.11 MiB -> 1.07+1.45=2.52 MiB
            // Binary: -10.01 MiB
            // TODO: What if the dependent word is in words?
            let mut romajis = if kanji_romanizer.is_romanizable(word) {
                let romajis = kanas_set
                    .iter()
                    .cloned()
                    .filter(|romaji| !kanji_romanizer.is_romanizable_to(word, romaji))
                    .collect::<Vec<_>>();
                if romajis.len() != kanas_set.len() {
                    if romajis.is_empty() {
                        // println!("romanizable: {word}");
                        romanizable_count += 1;
                        continue;
                    }
                    println!(
                        "partial: {word} -{} {kanas_set:?} -> {romajis:?}",
                        kanas_set.len() - romajis.len()
                    );
                    partial_romanizable_count += 1;
                } else {
                    println!("diff: {word} {kanas_set:?}");
                    diff_romanizable_count += 1;
                }
                romajis
            } else {
                println!("un: {word}");
                unromanizable_count += 1;
                kanas_set.into_iter().collect()
            };
            romajis.sort_unstable();

            if word.len() > max_len {
                max_len = word.len();
            }
            if word.len() == data::WORD_MAX_LEN {
                println!("Max len word: {word}");
            }
            {
                let max_len = romajis.iter().map(|s| s.len()).max().unwrap();
                if max_len > romaji_max_len {
                    romaji_max_len = max_len;
                }
                if max_len == data::WORD_ROMAJI_MAX_LEN {
                    println!("Max len romaji: {word} {romajis:?}");
                }
            }

            // write!(out_words, "\"{kanji}\",").unwrap();
            // if i != end {
            //     write!(out_words, "{word}\n").unwrap();
            // } else {
            //     write!(out_words, "{word}").unwrap();
            // }
            if i == 0 {
                write!(out_words, "{word}").unwrap();
            } else {
                write!(out_words, "\n{word}").unwrap();
            }

            // i != 0 && (c + 1) % 8 == 0
            // Natural align
            let ch = word.chars().next().unwrap() as u32;
            let ch2 = word.chars().nth(1).unwrap_or_default() as u32;
            if ch / 100 != range || range_c > 10 && ch2 / 100 != range_2 {
                if ch / 100 != range {
                    range = ch / 100;
                    range_c = 0;
                }
                range_2 = ch2 / 100;
                if i != 0 {
                    // out_words.write_all(b"\n").unwrap();
                    // out_words.write_all(b"\\\n").unwrap();
                    out_kanas.write_all(b"\n").unwrap();
                }
            } else {
                range_c += 1;
            }

            write!(
                out_kanas,
                "&[{}],",
                romajis
                    .into_iter()
                    .map(|romaji| format!("\"{}\"", romaji))
                    .collect::<Vec<_>>()
                    .join(",")
            )
            .unwrap();

            // c += 1;
        }
        // write!(out_words, "\n]").unwrap();
        // write!(out_words, "\\\n\"").unwrap();
        write!(out_kanas, "\n]").unwrap();

        println!("Words with duplicated romajis: {dup_count}");
        println!();
        println!("Romanizable words: {romanizable_count}");
        println!("Partial romanizable words: {partial_romanizable_count}");
        println!("Different romanizable words: {diff_romanizable_count}");
        println!("Unromanizable words: {unromanizable_count}");
        println!();
        println!("Max word length: {max_len}");
        assert_eq!(data::WORD_MAX_LEN, max_len);
        println!("Romaji max length: {romaji_max_len}");
        assert_eq!(data::WORD_ROMAJI_MAX_LEN, romaji_max_len);
    }

    #[test]
    fn kanji() {
        assert_eq!(
            data::kanji_romajis('日'),
            [
                "a", "aki", "bi", "chi", "he", "hi", "iru", "jitsu", "ka", "kou", "ku", "kusa",
                "nchi", "ni", "nichi", "nitsu", "su", "tachi"
            ]
        );

        let data = HepburnRomanizer::builder().kana(true).kanji(true).build();
        assert_eq!(data.romanize_vec("は"), vec![(3, "ha")]);
        assert_eq!(data.romanize_vec("ハハハ"), vec![(3, "ha")]);
        assert_eq!(data.romanize_vec("ジョジョ"), vec![(6, "jo")]);
        assert_eq!(data.romanize_vec("って"), vec![(6, "tte")]);
        assert_eq!(
            data.romanize_vec("日は"),
            [
                "a", "aki", "bi", "chi", "he", "hi", "iru", "jitsu", "ka", "kou", "ku", "kusa",
                "nchi", "ni", "nichi", "nitsu", "su", "tachi"
            ]
            .map(|romaji| (3, romaji))
        );
        assert_eq!(
            data.romanize_vec("今日"),
            vec![(3, "ima"), (3, "kin"), (3, "kon"), (3, "na")]
        );
    }

    #[test]
    fn word() {
        let data = HepburnRomanizer::builder().kana(true).word(true).build();
        assert_eq!(data.romanize_vec("は"), vec![(3, "ha")]);
        assert_eq!(data.romanize_vec("ハハハ"), vec![(3, "ha")]);
        assert_eq!(data.romanize_vec("ジョジョ"), vec![(6, "jo")]);
        assert_eq!(data.romanize_vec("って"), vec![(6, "tte")]);
        assert_eq!(data.romanize_vec("日は"), vec![]);
        assert_eq!(data.romanize_vec("今日"), vec![(6, "kyou")]);

        let data = HepburnRomanizer::builder()
            .kana(true)
            .kanji(true)
            .word(true)
            .build();
        assert_eq!(
            data.romanize_vec("今日"),
            vec![(6, "kyou"), (3, "ima"), (3, "kin"), (3, "kon"), (3, "na")]
        );
    }
}

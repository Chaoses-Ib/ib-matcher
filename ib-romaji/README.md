# ib-romaji
[![crates.io](https://img.shields.io/crates/v/ib-romaji.svg)](https://crates.io/crates/ib-romaji)
[![Documentation](https://docs.rs/ib-romaji/badge.svg)](https://docs.rs/ib-romaji)
[![License](https://img.shields.io/crates/l/ib-romaji.svg)](../LICENSE.txt)

A fast Japanese romanizer.

## Usage
```rust
use ib_romaji::HepburnRomanizer;

let romanizer = HepburnRomanizer::default();

let mut romajis = Vec::new();
romanizer.romanize_and_try_for_each("日本語", |len, romaji| {
    romajis.push((len, romaji));
    None::<()>
});
assert_eq!(romajis, vec![(9, "nippongo"), (3, "a"), (3, "aki"), (3, "bi"), (3, "chi"), (3, "he"), (3, "hi"), (3, "iru"), (3, "jitsu"), (3, "ka"), (3, "kou"), (3, "ku"), (3, "kusa"), (3, "nchi"), (3, "ni"), (3, "nichi"), (3, "nitsu"), (3, "su"), (3, "tachi")]);

assert_eq!(romanizer.romanize_vec("日本語"), vec![(9, "nippongo"), (3, "a"), (3, "aki"), (3, "bi"), (3, "chi"), (3, "he"), (3, "hi"), (3, "iru"), (3, "jitsu"), (3, "ka"), (3, "kou"), (3, "ku"), (3, "kusa"), (3, "nchi"), (3, "ni"), (3, "nichi"), (3, "nitsu"), (3, "su"), (3, "tachi")]);
```

## Comparison with other crates
- [kakasi: kakasi is a Rust library to transliterate hiragana, katakana and kanji (Japanese text) into rōmaji (Latin/Roman alphabet)](https://github.com/Theta-Dev/kakasi)

  `kakasi`'s dictionary is a bit outdated and it's licensed under GPL-3. While `ib-romaji` uses the latest JMdict and licensed under MIT. `ib-romaji` also supports querying all possible romajis of a word.

The following crates are kana (仮名) only. They don't support kanjis like `日本語`:
- [wana\_kana\_rust: Utility library for checking and converting between Japanese characters - Hiragana, Katakana - and Romaji](https://github.com/PSeitz/wana_kana_rust)
- [uzimith/romaji: Romaji-Kana transliterator in Rust](https://github.com/uzimith/romaji)
- [TianyiShi2001/romkan: A Romaji/Kana conversion library for Rust](https://github.com/TianyiShi2001/romkan)

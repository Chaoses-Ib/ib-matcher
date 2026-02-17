/*!
# Kanji romanization
## Handling of 々(noma)
々(noma) is basically a kanji repetition suffix.
See [々 - ウィクショナリー日本語版](https://ja.wiktionary.org/wiki/%E3%80%85) for details.

々 is partially supported via the word dictionary in v0.1, like `日々`.
"Full support was planned but not yet done. In my files only `日々は過ぎれど飯うまし` has it so..."

v0.2 added full support for single 々.
Besides supporting more 々 usage, this also reduced the word dictionary size
by 591 (0.69%) words and some word kanas.
The word dictionary is still kept for 連濁 words (292) and words containing two 々 (only 9).
*/

use ib_unicode::str::RoundCharBoundaryExt;

use crate::{HepburnRomanizer, Input, data};

pub const NOMA: char = '々';
pub const NOMA_STR: &str = "々";
pub const NOMA_ROMAJI: &str = "noma";

impl HepburnRomanizer {
    pub(crate) fn romanize_kanji_and_try_for_each<'h, S: Into<Input<'h>>, T>(
        &self,
        input: S,
        mut f: impl FnMut(usize, &'static str) -> Option<T>,
    ) -> Option<T> {
        let input = input.into();
        let s = input.as_ref();

        // let s = unsafe { str::from_utf8_unchecked(s) };
        if let Some(kanji) = s.chars().next() {
            if kanji != NOMA {
                // TODO: Binary search
                for romaji in data::kanji_romajis(kanji) {
                    // TODO: Always 3?
                    if let Some(result) = f(kanji.len_utf8(), romaji) {
                        return Some(result);
                    }
                }
            } else {
                // Noma is only used for kanji
                if input.start() >= data::KANJI_MIN_LEN {
                    let h = input.haystack();
                    let i = h.floor_char_boundary_ib(input.start() - 1);
                    let kanji = h[i..].chars().next().unwrap();
                    for romaji in data::kanji_romajis(kanji) {
                        if let Some(result) = f(NOMA.len_utf8(), romaji) {
                            return Some(result);
                        }
                    }
                }
                if let Some(result) = f(NOMA.len_utf8(), NOMA_ROMAJI) {
                    return Some(result);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noma() {
        let data = HepburnRomanizer::builder().kana(true).kanji(true).build();
        assert_eq!(data.romanize_vec("々"), vec![(3, NOMA_ROMAJI)]);

        // 水樹奈々
        assert_eq!(
            data.romanize_vec("奈々"),
            vec![
                (3, "dai"),
                (3, "ikan"),
                (3, "karanashi"),
                (3, "na"),
                (3, "nai")
            ]
        );
        assert_eq!(data.romanize_vec("々"), vec![(3, NOMA_ROMAJI)]);
        assert_eq!(
            data.romanize_vec(Input::new("奈々", 3)),
            vec![
                (3, "dai"),
                (3, "ikan"),
                (3, "karanashi"),
                (3, "na"),
                (3, "nai"),
                (3, NOMA_ROMAJI)
            ]
        );

        // Common words
        assert_eq!(
            data.romanize_vec(Input::new("日々", 3)),
            vec![
                (3, "a"),
                (3, "aki"),
                (3, "bi"),
                (3, "chi"),
                (3, "he"),
                (3, "hi"),
                (3, "iru"),
                (3, "jitsu"),
                (3, "ka"),
                (3, "kou"),
                (3, "ku"),
                (3, "kusa"),
                (3, "nchi"),
                (3, "ni"),
                (3, "nichi"),
                (3, "nitsu"),
                (3, "su"),
                (3, "tachi"),
                (3, NOMA_ROMAJI)
            ]
        );
        assert_eq!(
            data.romanize_vec(Input::new("時々", 3)),
            vec![
                (3, "aki"),
                (3, "doki"),
                (3, "ji"),
                (3, "to"),
                (3, "togi"),
                (3, "toki"),
                (3, NOMA_ROMAJI)
            ]
        );

        // Uncommon words
        // https://youtu.be/XnEYwt3Fkb4
        assert_eq!(
            data.romanize_vec(Input::new("眩々", 3)),
            vec![
                (3, "gen"),
                (3, "gensu"),
                (3, "kan"),
                (3, "kuramu"),
                (3, "kureru"),
                (3, "kurumeku"),
                (3, "mabayui"),
                (3, "mabushii"),
                (3, "madou"),
                (3, "mau"),
                (3, "memai"),
                (3, NOMA_ROMAJI)
            ]
        );

        // 意味が区切れる場合には用いられないが、慣用的に用いる場合もある。
        assert_eq!(
            data.romanize_vec(Input::new("結婚式々場", 9)),
            vec![(3, "nori"), (3, "shiki"), (3, NOMA_ROMAJI)]
        );
    }

    #[test]
    fn noma_word() {
        let data = HepburnRomanizer::default();
        assert_eq!(
            data.romanize_vec("馬鹿々々しい"),
            vec![
                (18, "bakabakashii"),
                (3, "ba"),
                (3, "ban"),
                (3, "ma"),
                (3, "me"),
                (3, "mo"),
                (3, "ta"),
                (3, "uma")
            ]
        );
    }
}

/*!
See [Pinyin](https://en.wikipedia.org/wiki/Pinyin) for what is pinyin.

## Features
- 支持以下拼音编码方案：
  - 简拼（“py”）
  - 全拼（“pinyin”）
  - 带声调全拼（“pin1yin1”）
  - Unicode（“pīnyīn”）
  - 智能 ABC 双拼
  - 拼音加加双拼
  - 微软双拼
  - 华宇双拼（紫光双拼）
  - 小鹤双拼
  - 自然码双拼
- 支持多音字。
- 支持 Unicode 辅助平面汉字。
*/

use std::ops::RangeInclusive;

mod data;
mod notation;

pub(super) type PinyinCombination = [u16; data::PINYIN_COMBINATION_LEN];

pub(super) struct PinyinRangeTable {
    range: RangeInclusive<u32>,
    /// Array of indices into `data::PINYINS` or `data::PINYIN_COMBINATIONS`.
    table: &'static [u16],
}

impl PinyinRangeTable {
    const MAX_RANGE: RangeInclusive<u32> = 0x3007..=0x30EDE;

    pub(super) const fn new(range: RangeInclusive<u32>, table: &'static [u16]) -> Self {
        PinyinRangeTable { range, table }
    }
}

use itertools::Itertools;
pub use notation::PinyinNotation;

type PinyinString = arraystring::ArrayString<arraystring::typenum::U7>;

#[cfg(not(feature = "inmut-data"))]
type OptionalPinyinStringArray = Option<Box<[PinyinString]>>;
#[cfg(feature = "inmut-data")]
type OptionalPinyinStringArray = std::sync::OnceLock<Box<[PinyinString]>>;

/// ## Memory usage
/// Per pinyin notation: 8 * 1514 ≈ 11.8 KiB.
/// - `Unicode` does not require extra memory.
/// - `AsciiFirstLetter` uses the same storage as `Ascii`.
///
/// ## Others
/// TODO: Optionally generate pinyin notation data at build time.
///
/// Less flexible at runtime.
///
/// TODO: Is row-major order more cache friendly?
///
/// Row-major order requires 7 * 8 * 1514 ≈ 83 KiB, dynamic columns are needed to reduce memory usage. And dynamic columns can also offer better cache locality.
///
/// TODO: Order pinyin by frequency to improve cache locality?
#[derive(Clone)]
pub struct PinyinData {
    #[cfg(not(feature = "inmut-data"))]
    inited_notations: PinyinNotation,
    #[cfg(feature = "inmut-data")]
    inited_notations: notation::AtomicPinyinNotation,

    ascii: OptionalPinyinStringArray,
    ascii_tone: OptionalPinyinStringArray,
    // ascii_first_letter: Option<Box<[u8]>>,
    diletter_abc: OptionalPinyinStringArray,
    diletter_jiajia: OptionalPinyinStringArray,
    diletter_microsoft: OptionalPinyinStringArray,
    diletter_thunisoft: OptionalPinyinStringArray,
    diletter_xiaohe: OptionalPinyinStringArray,
    diletter_zrm: OptionalPinyinStringArray,
}

impl PinyinData {
    pub fn new(notations: PinyinNotation) -> Self {
        #[cfg_attr(feature = "inmut-data", allow(unused_mut))]
        let mut pinyin_data = Self {
            inited_notations: PinyinNotation::Unicode.into(),
            ascii: Default::default(),
            ascii_tone: Default::default(),
            // ascii_first_letter: Default::default(),
            diletter_abc: Default::default(),
            diletter_jiajia: Default::default(),
            diletter_microsoft: Default::default(),
            diletter_thunisoft: Default::default(),
            diletter_xiaohe: Default::default(),
            diletter_zrm: Default::default(),
        };

        pinyin_data.init_notations(notations);
        pinyin_data
    }

    const fn notation(&self, notation: PinyinNotation) -> &OptionalPinyinStringArray {
        match notation {
            PinyinNotation::Unicode => unreachable!(),
            PinyinNotation::Ascii => &self.ascii,
            PinyinNotation::AsciiTone => &self.ascii_tone,
            PinyinNotation::AsciiFirstLetter => unreachable!(),
            PinyinNotation::DiletterAbc => &self.diletter_abc,
            PinyinNotation::DiletterJiajia => &self.diletter_jiajia,
            PinyinNotation::DiletterMicrosoft => &self.diletter_microsoft,
            PinyinNotation::DiletterThunisoft => &self.diletter_thunisoft,
            PinyinNotation::DiletterXiaohe => &self.diletter_xiaohe,
            PinyinNotation::DiletterZrm => &self.diletter_zrm,
            _ => unreachable!(),
        }
    }

    #[cfg(not(feature = "inmut-data"))]
    pub fn init_notations(&mut self, notations: PinyinNotation) {
        Self::init_notations_inner(self, notations)
    }

    #[cfg(feature = "inmut-data")]
    pub fn init_notations(&self, notations: PinyinNotation) {
        Self::init_notations_inner(self, notations)
    }

    fn init_notations_inner(
        // `self` must be the first parameter of an associated function
        #[cfg(not(feature = "inmut-data"))] this: &mut Self,
        #[cfg(feature = "inmut-data")] this: &Self,
        notations: PinyinNotation,
    ) {
        for notation in notations.iter() {
            match notation {
                PinyinNotation::Unicode => (),
                PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter => {
                    let init = || {
                        data::PINYINS
                            .iter()
                            .map(|py| notation::unicode_to_ascii(py))
                            .collect::<Vec<_>>()
                            .into_boxed_slice()
                    };
                    #[cfg(not(feature = "inmut-data"))]
                    this.ascii.get_or_insert_with(init);
                    #[cfg(feature = "inmut-data")]
                    this.ascii.get_or_init(init);
                }
                PinyinNotation::AsciiTone => {
                    let init = || {
                        data::PINYINS
                            .iter()
                            .map(|py| notation::unicode_to_ascii_tone(py))
                            .collect::<Vec<_>>()
                            .into_boxed_slice()
                    };
                    #[cfg(not(feature = "inmut-data"))]
                    this.ascii_tone.get_or_insert_with(init);
                    #[cfg(feature = "inmut-data")]
                    this.ascii_tone.get_or_init(init);
                }
                _ => {
                    this.init_notations(PinyinNotation::Ascii);

                    let init = || {
                        #[cfg(not(feature = "inmut-data"))]
                        let ascii = this.ascii.as_ref().unwrap();
                        #[cfg(feature = "inmut-data")]
                        let ascii = this.ascii.get().unwrap();

                        let map = notation::ascii_map_fn(notation);
                        ascii
                            .iter()
                            .map(|py| map(py))
                            .collect::<Vec<_>>()
                            .into_boxed_slice()
                    };
                    #[cfg(not(feature = "inmut-data"))]
                    match notation {
                        PinyinNotation::DiletterAbc => &mut this.diletter_abc,
                        PinyinNotation::DiletterJiajia => &mut this.diletter_jiajia,
                        PinyinNotation::DiletterMicrosoft => &mut this.diletter_microsoft,
                        PinyinNotation::DiletterThunisoft => &mut this.diletter_thunisoft,
                        PinyinNotation::DiletterXiaohe => &mut this.diletter_xiaohe,
                        PinyinNotation::DiletterZrm => &mut this.diletter_zrm,
                        _ => unreachable!(),
                    }
                    .get_or_insert_with(init);
                    #[cfg(feature = "inmut-data")]
                    this.notation(notation).get_or_init(init);
                }
            }
        }

        #[cfg(not(feature = "inmut-data"))]
        use core::ops::BitOrAssign;
        this.inited_notations.bitor_assign(notations);
    }

    pub fn inited_notations(&self) -> PinyinNotation {
        self.inited_notations.clone().into()
    }

    fn get_pinyin_index(c: char) -> Option<u16> {
        if PinyinRangeTable::MAX_RANGE.contains(&(c as u32)) {
            for range in &data::PINYIN_RANGE_TABLES {
                if range.range.contains(&(c as u32)) {
                    return match range.table[(c as u32 - range.range.start()) as usize] {
                        u16::MAX => None,
                        i => Some(i),
                    };
                }
            }
        }
        None
    }

    fn pinyin_combination(index: u16) -> impl Iterator<Item = &'static u16> {
        data::PINYIN_COMBINATIONS[index as usize]
            .iter()
            .take_while(|&&i| i != u16::MAX)
    }

    fn pinyin(&self, index: u16) -> Pinyin<'_> {
        Pinyin { data: self, index }
    }

    pub fn iter(&self) -> impl Iterator<Item = Pinyin<'_>> {
        (0..data::PINYINS.len() as u16).map(|i| self.pinyin(i))
    }

    /// Prefer [`PinyinData::get_pinyins_and_for_each`] and [`PinyinData::get_pinyins_and_try_for_each`] if applicable.
    ///
    /// ## Performance
    /// Do not use this method in performance-critical code. The `Box` wouldn't be optimized away even with `#[inline(always)]`.
    pub fn get_pinyins<'a>(&'a self, c: char) -> Box<dyn Iterator<Item = Pinyin<'a>> + 'a> {
        if let Some(i) = Self::get_pinyin_index(c) {
            if i < data::PINYINS.len() as u16 {
                Box::new([self.pinyin(i)].into_iter())
            } else {
                let i = i - data::PINYINS.len() as u16;
                Box::new(Self::pinyin_combination(i).map(|&i| self.pinyin(i)))
            }
        } else {
            Box::new([].into_iter())
        }
    }

    pub fn get_pinyins_and_for_each(&self, c: char, mut f: impl FnMut(Pinyin)) {
        if let Some(i) = Self::get_pinyin_index(c) {
            if i < data::PINYINS.len() as u16 {
                f(self.pinyin(i));
            } else {
                let i = i - data::PINYINS.len() as u16;
                Self::pinyin_combination(i).for_each(|&i| f(self.pinyin(i)));
            }
        }
    }

    pub fn get_pinyins_and_try_for_each<T>(
        &self,
        c: char,
        mut f: impl FnMut(Pinyin) -> Option<T>,
    ) -> Option<T> {
        if let Some(i) = Self::get_pinyin_index(c) {
            if i < data::PINYINS.len() as u16 {
                f(self.pinyin(i))
            } else {
                let i = i - data::PINYINS.len() as u16;
                for &i in Self::pinyin_combination(i) {
                    if let Some(v) = f(self.pinyin(i)) {
                        return Some(v);
                    }
                }
                None
            }
        } else {
            None
        }
    }

    /// Match pinyin of the given notation in haystack.
    pub fn match_pinyin<'a: 'h, 'h>(
        &'a self,
        notation: PinyinNotation,
        haystack: &'h str,
    ) -> impl Iterator<Item = &'a str> + 'h {
        debug_assert_eq!(notation.bits().count_ones(), 1);
        debug_assert!(self.inited_notations().contains(notation));

        self.iter()
            .map(move |pinyin| pinyin.notation(notation).unwrap())
            .filter(move |py| haystack.starts_with(py))
            .dedup()
    }

    /// Match pinyin of the given notation in haystack, optionally allowing partial matches.
    ///
    /// ## Returns
    /// `(pinyin, partial)`
    /// - If `partial` is `true`, `haystack.len() < pinyin.len()`.
    pub fn match_pinyin_partial<'a: 'h, 'h>(
        &'a self,
        notation: PinyinNotation,
        haystack: &'h str,
        partial: bool,
    ) -> impl Iterator<Item = (&'a str, bool)> + 'h {
        debug_assert_eq!(notation.bits().count_ones(), 1);
        debug_assert!(self.inited_notations().contains(notation));

        self.iter()
            .map(move |pinyin| pinyin.notation(notation).unwrap())
            .dedup()
            .filter_map(move |py| {
                if haystack.starts_with(py) {
                    Some((py, false))
                } else if partial && py.starts_with(haystack) {
                    debug_assert!(haystack.len() < py.len());
                    Some((py, true))
                } else {
                    None
                }
            })
    }
}

pub struct Pinyin<'a> {
    data: &'a PinyinData,
    index: u16,
}

impl<'a> Pinyin<'a> {
    pub fn notation(&self, notation: PinyinNotation) -> Option<&'a str> {
        debug_assert_eq!(notation.bits().count_ones(), 1);

        let i = self.index as usize;

        let get = |pinyins: &'a OptionalPinyinStringArray| {
            #[cfg(not(feature = "inmut-data"))]
            let notation = pinyins.as_ref().map(|pinyins| pinyins[i].as_str());
            #[cfg(feature = "inmut-data")]
            let notation = pinyins.get().map(|pinyins| pinyins[i].as_str());
            notation
        };

        match notation {
            PinyinNotation::Unicode => Some(data::PINYINS[i]),
            PinyinNotation::AsciiFirstLetter => {
                get(&self.data.ascii).map(|ascii| unsafe { ascii.get_unchecked(..1) })
            }
            _ => get(self.data.notation(notation)),
        }
    }

    /// Require `PinyinNotation::Ascii`.
    pub fn initial_final(&self) -> Option<(&str, &str)> {
        self.notation(PinyinNotation::Ascii)
            .map(Self::split_initial_final)
    }

    fn split_initial_final(ascii: &str) -> (&str, &str) {
        debug_assert!(ascii.is_ascii());

        // TODO: as_ascii_unchecked
        let mut chars = ascii.chars();
        ascii.split_at(match chars.next().unwrap() {
            'a' | 'e' | 'i' | 'o' | 'u' | 'v' => 0,
            'z' | 'c' | 's' => {
                if chars.next().unwrap() == 'h' {
                    2
                } else {
                    1
                }
            }
            _ => 1,
        })
    }
}

impl core::fmt::Debug for Pinyin<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = f.debug_struct("Pinyin");
        for (name, notation) in PinyinNotation::all().iter_names() {
            if let Some(py) = self.notation(notation) {
                s.field(name, &py);
            }
        }
        s.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinyin_max_len() {
        let max = data::PINYINS.iter().map(|p| p.len()).max().unwrap();
        assert_eq!(max, 7);
    }

    #[test]
    fn pinyin_range_tables() {
        // `Option::<T>::unwrap` is not yet stable as a const fn
        // const MIN_START: u32 = data::PINYIN_RANGE_TABLES
        //     .iter()
        //     .map(|range| *range.range.start())
        //     .min()
        //     .unwrap();
        // const MAX_END: u32 = data::PINYIN_RANGE_TABLES
        //     .iter()
        //     .map(|range| *range.range.end())
        //     .max()
        //     .unwrap();

        let min_start = data::PINYIN_RANGE_TABLES
            .iter()
            .map(|range| *range.range.start())
            .min()
            .unwrap();
        let max_end = data::PINYIN_RANGE_TABLES
            .iter()
            .map(|range| *range.range.end())
            .max()
            .unwrap();
        assert_eq!(min_start..=max_end, PinyinRangeTable::MAX_RANGE)
    }

    #[test]
    fn get_pinyins() {
        let data = PinyinData::new(PinyinNotation::all());

        assert_eq!(data.get_pinyins('中').count(), 2);

        for pinyin in data.get_pinyins('中') {
            println!("{:?}", pinyin);

            for notation in PinyinNotation::all().iter() {
                assert!(pinyin.notation(notation).is_some_and(|py| !py.is_empty()));
            }
        }
    }
}

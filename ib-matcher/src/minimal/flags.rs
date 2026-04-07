use crate::pinyin::PinyinNotation;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MatcherFlags(u32);

bitflags::bitflags! {
    impl MatcherFlags: u32 {
        /// e.g. "pīn", "yīn"
        const Unicode = 0x8;

        /// 全拼
        ///
        /// e.g. "pin", "yin"
        ///
        /// See [全拼](https://zh.wikipedia.org/wiki/全拼) for details.
        #[doc(alias = "全拼")]
        const Ascii = 0x2;

        /// 带声调全拼
        ///
        /// The tone digit is in `1..=5`. See [tones](https://en.wikipedia.org/wiki/Pinyin#Tones) for details.
        ///
        /// e.g. "pin1", "yin1"
        #[doc(alias = "带声调全拼")]
        const AsciiTone = 0x4;

        /// 简拼
        ///
        /// e.g. "p", "y"
        ///
        /// See [简拼](https://zh.wikipedia.org/wiki/简拼) for details.
        #[doc(alias = "简拼")]
        const AsciiFirstLetter = 0x1;

        /// 智能 ABC 双拼
        ///
        /// See [智能ABC输入法](https://zh.wikipedia.org/wiki/智能ABC输入法#双拼方案) for details.
        #[doc(alias = "智能ABC双拼")]
        const DiletterAbc = 0x10;

        /// 拼音加加双拼
        ///
        /// See [拼音加加](https://zh.wikipedia.org/wiki/拼音加加#双拼方案) for details.
        #[doc(alias = "拼音加加双拼")]
        const DiletterJiajia = 0x20;

        /// 微软双拼
        ///
        /// See [微软拼音输入法](https://zh.wikipedia.org/wiki/微软拼音输入法#双拼方案) for details.
        #[doc(alias = "微软双拼")]
        const DiletterMicrosoft = 0x40;

        /// 华宇双拼（紫光双拼）
        ///
        /// See [华宇拼音输入法](https://zh.wikipedia.org/wiki/华宇拼音输入法#双拼方案) for details.
        #[doc(alias("华宇双拼", "紫光双拼"))]
        const DiletterThunisoft = 0x80;

        /// 小鹤双拼
        ///
        /// See [小鹤双拼](https://flypy.com/) for details.
        #[doc(alias = "小鹤双拼")]
        const DiletterXiaohe = 0x100;

        /// 自然码双拼
        ///
        /// See [自然码](https://zh.wikipedia.org/zh-cn/自然码) for details.
        #[doc(alias = "自然码双拼")]
        const DiletterZrm = 0x200;

        const PinyinNotationMask = 0xFFF;

        /// 允许部分拼音匹配
        ///
        /// 允许用不完整的拼音匹配，例如用 "su" 匹配 "算"
        const PatternPartial = 0x40000000;
    }
}

impl From<PinyinNotation> for MatcherFlags {
    fn from(value: PinyinNotation) -> Self {
        Self(value.bits())
    }
}

impl Into<PinyinNotation> for MatcherFlags {
    fn into(self) -> PinyinNotation {
        PinyinNotation::from_bits_truncate(self.bits() & Self::PinyinNotationMask.bits())
    }
}

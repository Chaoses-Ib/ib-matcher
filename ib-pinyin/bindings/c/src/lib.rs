#[diplomat::bridge]
mod ffi {
    use std::ffi::CStr;

    use ::ib_pinyin::minimal::{self, Match, MatcherFlags};
    use widestring::{U16CStr, U16Str, U32CStr, U32Str};

    /// https://github.com/rust-diplomat/diplomat/issues/392
    #[allow(non_camel_case_types)]
    #[diplomat::opaque]
    pub struct ib_pinyin;

    impl ib_pinyin {
        pub fn is_match_u8(pattern: &str, haystack: &str, pinyin_notations: u32) -> bool {
            minimal::is_pinyin_match(
                pattern,
                haystack,
                MatcherFlags::from_bits_truncate(pinyin_notations),
            )
        }

        /// TODO: Lossy decoding?
        pub fn is_match_u8c(pattern: &u8, haystack: &u8, pinyin_notations: u32) -> bool {
            (|| -> Result<bool, std::str::Utf8Error> {
                Ok(Self::is_match_u8(
                    unsafe { CStr::from_ptr(pattern as *const _ as *const i8) }.to_str()?,
                    unsafe { CStr::from_ptr(haystack as *const _ as *const i8) }.to_str()?,
                    pinyin_notations,
                ))
            })()
            .unwrap_or(false)
        }

        pub fn is_match_u16(
            pattern: &u16,
            pattern_len: usize,
            haystack: &u16,
            haystack_len: usize,
            pinyin_notations: u32,
        ) -> bool {
            minimal::is_pinyin_match_u16(
                unsafe { U16Str::from_ptr(pattern as *const u16, pattern_len) },
                unsafe { U16Str::from_ptr(haystack as *const u16, haystack_len) },
                MatcherFlags::from_bits_truncate(pinyin_notations),
            )
        }

        pub fn is_match_u16c(pattern: &u16, haystack: &u16, pinyin_notations: u32) -> bool {
            minimal::is_pinyin_match_u16(
                unsafe { U16CStr::from_ptr_str(pattern as *const u16) }.as_ustr(),
                unsafe { U16CStr::from_ptr_str(haystack as *const u16) }.as_ustr(),
                MatcherFlags::from_bits_truncate(pinyin_notations),
            )
        }

        pub fn is_match_u32(
            pattern: &u32,
            pattern_len: usize,
            haystack: &u32,
            haystack_len: usize,
            pinyin_notations: u32,
        ) -> bool {
            minimal::is_pinyin_match_u32(
                unsafe { U32Str::from_ptr(pattern as *const u32, pattern_len) },
                unsafe { U32Str::from_ptr(haystack as *const u32, haystack_len) },
                MatcherFlags::from_bits_truncate(pinyin_notations),
            )
        }

        pub fn is_match_u32c(pattern: &u32, haystack: &u32, pinyin_notations: u32) -> bool {
            minimal::is_pinyin_match_u32(
                unsafe { U32CStr::from_ptr_str(pattern as *const u32) }.as_ustr(),
                unsafe { U32CStr::from_ptr_str(haystack as *const u32) }.as_ustr(),
                MatcherFlags::from_bits_truncate(pinyin_notations),
            )
        }

        fn match_to_u64(m: Option<Match>) -> u64 {
            if let Some(m) = m {
                m.start() as u64 | (m.end() as u64) << 32
            } else {
                u64::MAX
            }
        }

        /// ## Returns
        /// - `start | (end << 32)` if a match is found.
        /// - `u64::MAX` (`-1`) if no match is found.
        pub fn find_match_u8(pattern: &str, haystack: &str, pinyin_notations: u32) -> u64 {
            Self::match_to_u64(minimal::find_pinyin_match(
                pattern,
                haystack,
                MatcherFlags::from_bits_truncate(pinyin_notations),
            ))
        }

        pub fn find_match_u8c(pattern: &u8, haystack: &u8, pinyin_notations: u32) -> u64 {
            (|| -> Result<u64, std::str::Utf8Error> {
                Ok(Self::find_match_u8(
                    unsafe { CStr::from_ptr(pattern as *const _ as *const i8) }.to_str()?,
                    unsafe { CStr::from_ptr(haystack as *const _ as *const i8) }.to_str()?,
                    pinyin_notations,
                ))
            })()
            .unwrap_or(u64::MAX)
        }

        pub fn find_match_u16(
            pattern: &u16,
            pattern_len: usize,
            haystack: &u16,
            haystack_len: usize,
            pinyin_notations: u32,
        ) -> u64 {
            Self::match_to_u64(minimal::find_pinyin_match_u16(
                unsafe { U16Str::from_ptr(pattern as *const u16, pattern_len) },
                unsafe { U16Str::from_ptr(haystack as *const u16, haystack_len) },
                MatcherFlags::from_bits_truncate(pinyin_notations),
            ))
        }

        pub fn find_match_u16c(pattern: &u16, haystack: &u16, pinyin_notations: u32) -> u64 {
            Self::match_to_u64(minimal::find_pinyin_match_u16(
                unsafe { U16CStr::from_ptr_str(pattern as *const u16) }.as_ustr(),
                unsafe { U16CStr::from_ptr_str(haystack as *const u16) }.as_ustr(),
                MatcherFlags::from_bits_truncate(pinyin_notations),
            ))
        }

        pub fn find_match_u32(
            pattern: &u32,
            pattern_len: usize,
            haystack: &u32,
            haystack_len: usize,
            pinyin_notations: u32,
        ) -> u64 {
            Self::match_to_u64(minimal::find_pinyin_match_u32(
                unsafe { U32Str::from_ptr(pattern as *const u32, pattern_len) },
                unsafe { U32Str::from_ptr(haystack as *const u32, haystack_len) },
                MatcherFlags::from_bits_truncate(pinyin_notations),
            ))
        }

        pub fn find_match_u32c(pattern: &u32, haystack: &u32, pinyin_notations: u32) -> u64 {
            Self::match_to_u64(minimal::find_pinyin_match_u32(
                unsafe { U32CStr::from_ptr_str(pattern as *const u32) }.as_ustr(),
                unsafe { U32CStr::from_ptr_str(haystack as *const u32) }.as_ustr(),
                MatcherFlags::from_bits_truncate(pinyin_notations),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use ::ib_pinyin::pinyin::PinyinNotation;
    use widestring::{u16cstr, u16str};

    use super::ffi::*;

    #[test]
    fn is_match_u16() {
        let pattern = u16str!("zuo");
        let haystack = u16str!("协作");
        assert!(ib_pinyin::is_match_u16(
            unsafe { &*pattern.as_ptr() },
            pattern.len(),
            unsafe { &*haystack.as_ptr() },
            haystack.len(),
            (PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter).bits()
        ));

        let pattern = u16str!("wszpx");
        let haystack = u16str!("我是只螃蟹");
        assert!(ib_pinyin::is_match_u16(
            unsafe { &*pattern.as_ptr() },
            pattern.len(),
            unsafe { &*haystack.as_ptr() },
            haystack.len(),
            (PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter).bits()
        ));
    }

    #[test]
    fn is_match_u16c() {
        let pattern = u16cstr!("zuo");
        let haystack = u16cstr!("协作");
        assert!(ib_pinyin::is_match_u16c(
            unsafe { &*pattern.as_ptr() },
            unsafe { &*haystack.as_ptr() },
            (PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter).bits()
        ));
    }
}

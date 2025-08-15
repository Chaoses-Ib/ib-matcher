//! ## Performance
//! With default `release` profile, using `Input` instead of `&HaystackStr` is 3~5% slower (without using Bon), while with `lto = "fat"` and `codegen-units = 1` using `Input` is 3~5% faster, well...
use bon::Builder;

use crate::matcher::encoding::EncodedStr;

#[derive(Builder, Clone)]
pub struct Input<'h, HaystackStr = str>
where
    HaystackStr: EncodedStr + ?Sized,
{
    #[builder(start_fn)]
    pub(crate) haystack: &'h HaystackStr,
    // #[builder(default = haystack.is_ascii())]
    // pub(crate) is_ascii: bool,
    /// The haystack does not include the real start of the haystack. Akin to POSIX `REG_NOTBOL` and PCRE `PCRE_NOTBOL`.
    #[builder(default = false)]
    pub(crate) no_start: bool,
}

impl<'h, HaystackStr> From<&'h HaystackStr> for Input<'h, HaystackStr>
where
    HaystackStr: EncodedStr + ?Sized,
{
    #[inline]
    fn from(haystack: &'h HaystackStr) -> Self {
        // Input::builder(haystack).build()
        Input {
            haystack,
            no_start: false,
        }
    }
}

#[cfg(feature = "regex-automata")]
impl<'h> From<crate::regex::Input<'h>> for Input<'h, str> {
    #[inline]
    fn from(input: crate::regex::Input<'h>) -> Self {
        debug_assert!(str::from_utf8(input.haystack()).is_ok());
        Input {
            haystack: unsafe { std::mem::transmute(str::from_utf8_unchecked(input.haystack())) },
            no_start: false,
        }
    }
}

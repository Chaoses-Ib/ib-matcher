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
impl<'h> Input<'h, str> {
    /// Note that:
    /// - `span` can limit the range, but the retuened [`Match`](super::Match) from [`IbMatcher`](super::IbMatcher) will start from `span.start`. You need to call [`m.offset(input.start())`](super::Match::offset) manually if the offsets matter in your case.
    /// - `anchored` and `earliest` will be ignored.
    #[inline]
    pub fn from_regex(input: &crate::regex::Input<'h>) -> Self {
        let haystack = &input.haystack()[input.get_span()];
        debug_assert!(str::from_utf8(haystack).is_ok());
        Input {
            haystack: unsafe { std::mem::transmute(str::from_utf8_unchecked(haystack)) },
            no_start: input.start() != 0,
        }
    }
}

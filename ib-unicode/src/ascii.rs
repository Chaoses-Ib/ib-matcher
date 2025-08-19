/// Returns the index of the first non-ASCII byte in this byte string (if
/// any such indices exist). Specifically, it returns the index of the
/// first byte with a value greater than or equal to `0x80`.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use ib_unicode::ascii::find_non_ascii_byte;
///
/// assert_eq!(Some(3), find_non_ascii_byte(b"abc\xff"));
/// assert_eq!(None, find_non_ascii_byte(b"abcde"));
/// assert_eq!(Some(0), find_non_ascii_byte("😀".as_bytes()));
/// ```
#[cfg_attr(feature = "perf-ascii", inline)]
pub fn find_non_ascii_byte(b: &[u8]) -> Option<usize> {
    #[cfg(not(feature = "perf-ascii"))]
    return b.iter().position(|&b| b > 0x7F);
    #[cfg(feature = "perf-ascii")]
    // sse2 (128) on x86_64, usize chunk on others
    bstr::ByteSlice::find_non_ascii_byte(b)
}

/// Search for the first occurrence of two possible bytes in a haystack.
///
/// This returns the index corresponding to the first occurrence of one of the
/// needle bytes in `haystack`, or `None` if one is not found. If an index is
/// returned, it is guaranteed to be less than `haystack.len()`.
///
/// While this is semantically the same as something like
/// `haystack.iter().position(|&b| b == needle1 || b == needle2)`, this routine
/// will attempt to use highly optimized vector operations that can be an order
/// of magnitude faster (or more).
///
/// # Example
///
/// This shows how to find the first position of one of two possible bytes in a
/// haystack.
///
/// ```
/// use ib_unicode::ascii::find_byte2;
///
/// let haystack = b"the quick brown fox";
/// assert_eq!(find_byte2(haystack, b'k', b'q'), Some(4));
/// ```
pub fn find_byte2(haystack: &[u8], needle1: u8, needle2: u8) -> Option<usize> {
    #[cfg(not(feature = "perf-find"))]
    return haystack.iter().position(|&b| b == needle1 || b == needle2);
    #[cfg(feature = "perf-find")]
    // sse2/avx2 (128) on x86_64
    memchr::memchr2(needle1, needle2, haystack)
}

#[cfg_attr(feature = "perf-ascii", inline)]
pub fn find_byte2_or_non_ascii_byte(haystack: &[u8], needle1: u8, needle2: u8) -> Option<usize> {
    // TODO: Opt
    // match (
    //     find_non_ascii_byte(haystack),
    //     find_byte2(haystack, needle1, needle2),
    // ) {
    //     (Some(m1), Some(m2)) => Some(m1.min(m2)),
    //     (Some(m1), None) => Some(m1),
    //     (None, Some(m2)) => Some(m2),
    //     (None, None) => None,
    // }

    // find_non_ascii_byte() is much faster than find_byte2() (2.3 vs 4.4ns)
    if let Some(m) = find_non_ascii_byte(haystack) {
        if let Some(m2) = find_byte2(unsafe { haystack.get_unchecked(..m) }, needle1, needle2) {
            Some(m2)
        } else {
            Some(m)
        }
    } else {
        find_byte2(haystack, needle1, needle2)
    }
}

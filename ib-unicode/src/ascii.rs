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

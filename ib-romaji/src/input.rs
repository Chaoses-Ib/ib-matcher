/**
Unfortunately, Japanese is highly contextual, surrounding charcaters
are needed for accurate romanization.
This struct can keep surrounding charcaters by storing the entire haystack
and the start offset.
*/
#[derive(Clone, Copy, Debug)]
pub struct Input<'h> {
    haystack: &'h str,
    start: usize,
}

impl<'h> Input<'h> {
    #[inline]
    pub fn new<H: ?Sized + AsRef<str>>(haystack: &'h H, start: usize) -> Self {
        Self {
            haystack: haystack.as_ref(),
            start,
        }
    }

    #[inline]
    pub fn haystack(&self) -> &'h str {
        self.haystack
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }
}

impl<'h, H: ?Sized + AsRef<str>> From<&'h H> for Input<'h> {
    fn from(haystack: &'h H) -> Self {
        Self::new(haystack, 0)
    }
}

impl<'h> AsRef<str> for Input<'h> {
    fn as_ref(&self) -> &'h str {
        &self.haystack[self.start..]
    }
}

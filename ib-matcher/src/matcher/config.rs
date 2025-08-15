use std::{
    marker::PhantomPinned,
    mem::{transmute, MaybeUninit},
    ops::Deref,
    sync::Arc,
};

use crate::matcher::{pattern::Pattern, IbMatcher, MatchConfig};

pub(crate) struct IbMatcherWithConfig<'a> {
    matcher: MaybeUninit<IbMatcher<'a>>,
    /// [`IbMatcher`] may have reference to this config due to `shallow_clone()`, i.e. self-references.
    /// We must keep it alive and not move it.
    /// That's also the reason why we wrap it into `Arc`.
    config: MatchConfig<'a>,
    _pin: PhantomPinned,
}

impl<'a> IbMatcherWithConfig<'a> {
    pub fn with_config<'p>(
        pattern: impl Into<Pattern<'p, str>>,
        config: MatchConfig<'a>,
    ) -> Arc<Self> {
        let mut this = Arc::new(Self {
            matcher: MaybeUninit::uninit(),
            config,
            _pin: PhantomPinned,
        });

        // `shallow_clone()` requires `config` cannot be moved
        let config: MatchConfig<'static> = unsafe { transmute(this.config.shallow_clone()) };
        let matcher = IbMatcher::with_config(pattern, config);
        unsafe {
            Arc::get_mut(&mut this)
                .unwrap_unchecked()
                .matcher
                .write(matcher)
        };

        this
    }
}

impl<'a> Deref for IbMatcherWithConfig<'a> {
    type Target = IbMatcher<'a>;

    fn deref(&self) -> &Self::Target {
        unsafe { self.matcher.assume_init_ref() }
    }
}

impl Drop for IbMatcherWithConfig<'_> {
    fn drop(&mut self) {
        // `with_config()` is infallible
        unsafe { self.matcher.assume_init_drop() };
    }
}

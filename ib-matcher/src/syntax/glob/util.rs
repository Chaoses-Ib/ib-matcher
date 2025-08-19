use regex_syntax::hir::{Hir, Look};

use crate::syntax::glob::{GlobPathToken, PathSeparator, WildcardPathToken};

pub(crate) enum SurroundingHandleToken {
    Any,
    Star,
    SepUnix,
    SepWin,
    Unwild,
}

impl From<WildcardPathToken> for SurroundingHandleToken {
    fn from(token: WildcardPathToken) -> Self {
        match token {
            WildcardPathToken::Any => Self::Any,
            WildcardPathToken::Star | WildcardPathToken::GlobStar => Self::Star,
            WildcardPathToken::SepUnix => Self::SepUnix,
            WildcardPathToken::SepWin => Self::SepWin,
            WildcardPathToken::Text => Self::Unwild,
        }
    }
}

impl From<GlobPathToken> for SurroundingHandleToken {
    fn from(token: GlobPathToken) -> Self {
        match token {
            GlobPathToken::Any => Self::Any,
            GlobPathToken::Star | GlobPathToken::GlobStar => Self::Star,
            GlobPathToken::SepUnix => Self::SepUnix,
            GlobPathToken::SepWin => Self::SepWin,
            GlobPathToken::Text | GlobPathToken::Class => Self::Unwild,
        }
    }
}

pub struct SurroundingWildcardHandler {
    leading_wildcard: bool,
    leading_star: bool,
    trailing_wildcards: usize,
    trailing_star: bool,
    sep: PathSeparator,
    seped: bool,
}

impl SurroundingWildcardHandler {
    pub fn new(pattern_separator: PathSeparator) -> Self {
        Self {
            leading_wildcard: false,
            leading_star: false,
            trailing_wildcards: 0,
            trailing_star: false,
            sep: pattern_separator,
            seped: false,
        }
    }
}

impl SurroundingWildcardHandler {
    pub fn skip<'p>(
        &mut self,
        token: impl Into<SurroundingHandleToken>,
        hirs: &mut Vec<Hir>,
        lex: &logos::Lexer<'p, impl logos::Logos<'p, Source = str>>,
    ) -> bool {
        let mut sep = || {
            // Insert StartLF if leading_wildcard
            if !self.leading_star && self.leading_wildcard {
                hirs.insert(0, Hir::look(Look::StartLF));
                // leading_wildcard will never be true again if hirs is not empty
            }
            self.leading_wildcard = false;
            self.leading_star = false;
            self.trailing_wildcards = 0;
            self.seped = true;
        };
        match token.into() {
            SurroundingHandleToken::Any => {
                // `?` is also treated as anchor, but not skipped
                if hirs.is_empty() {
                    self.leading_wildcard = true;
                }
                self.trailing_wildcards = 1;
            }
            SurroundingHandleToken::Star => {
                if hirs.is_empty() {
                    self.leading_wildcard = true;
                    self.leading_star = true;
                    return true;
                }
                self.trailing_wildcards += 1;
                if lex.remainder().is_empty() {
                    self.trailing_star = true;
                    return true;
                }
            }
            SurroundingHandleToken::SepUnix if self.sep.is_unix_or_any() => sep(),
            SurroundingHandleToken::SepWin if self.sep.is_windows_or_any() => sep(),
            SurroundingHandleToken::Unwild
            | SurroundingHandleToken::SepUnix
            | SurroundingHandleToken::SepWin => self.trailing_wildcards = 0,
        }
        false
    }

    fn insert_anchors_common(&self, hirs: &mut Vec<Hir>, sep: bool) {
        let start = || Hir::look(if sep { Look::StartLF } else { Look::Start });
        let end = || Hir::look(if sep { Look::EndLF } else { Look::End });

        // Unanchored search has implicit leading and trailing star.
        // We cancel them by anchors.
        match (self.leading_star, self.trailing_star) {
            // *a*
            (true, true) => (),
            // a*
            (false, true) => {
                // Strip trailing wildcards
                // hirs.truncate(
                //     hirs.len()
                //         - hirs
                //             .iter()
                //             .rev()
                //             .take_while(|hir| !matches!(hir.kind(), HirKind::Literal(_)))
                //             .count(),
                // );
                // while let Some(_) = hirs.pop_if(|hir| !matches!(hir.kind(), HirKind::Literal(_))) {}
                hirs.truncate(hirs.len() - (self.trailing_wildcards - 1));

                // Less used, reserving and replacing maybe not worth
                hirs.insert(0, start())
            }
            // *a
            (true, false) => hirs.push(end()),
            // ?a || a?
            (false, false) if self.leading_wildcard || self.trailing_wildcards != 0 => {
                if !self.seped {
                    hirs.insert(0, start());
                }
                hirs.push(end());
            }
            // a
            (false, false) => (),
        }
    }

    pub fn insert_anchors(&self, hirs: &mut Vec<Hir>) {
        self.insert_anchors_common(hirs, true);
    }
}

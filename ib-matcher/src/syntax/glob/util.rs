use regex_syntax::hir::{Hir, Look};

use crate::syntax::glob::WildcardToken;

#[derive(Default)]
pub struct SurroundingWildcardHandler {
    leading_wildcard: bool,
    leading_star: bool,
    trailing_wildcards: usize,
    trailing_star: bool,
}

impl SurroundingWildcardHandler {
    pub fn skip<'p>(
        &mut self,
        token: WildcardToken,
        hirs: &[Hir],
        lex: &logos::Lexer<'p, impl logos::Logos<'p, Source = str>>,
    ) -> bool {
        match token {
            WildcardToken::Any => {
                // `?` is also treated as anchor, but not skipped
                if hirs.is_empty() {
                    self.leading_wildcard = true;
                }
                self.trailing_wildcards = 1;
            }
            WildcardToken::Star => {
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
            WildcardToken::Text => self.trailing_wildcards = 0,
        }
        false
    }

    pub fn insert_anchors(&self, hirs: &mut Vec<Hir>) {
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
                hirs.insert(0, Hir::look(Look::Start))
            }
            // *a
            (true, false) => hirs.push(Hir::look(Look::End)),
            // ?a || a?
            (false, false) if self.leading_wildcard || self.trailing_wildcards != 0 => {
                hirs.insert(0, Hir::look(Look::Start));
                hirs.push(Hir::look(Look::End));
            }
            // a
            (false, false) => (),
        }
    }
}

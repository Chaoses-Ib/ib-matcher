use regex_syntax::hir::{Hir, Look};

use crate::syntax::glob::WildcardToken;

#[derive(Default)]
pub struct SurroundingWildcardHandler {
    leading_wildcard: bool,
    trailing_wildcards: usize,
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
                    return true;
                }
                self.trailing_wildcards += 1;
                if lex.remainder().is_empty() {
                    return true;
                }
            }
            WildcardToken::Text => self.trailing_wildcards = 0,
        }
        false
    }

    pub fn insert_anchors(&self, hirs: &mut Vec<Hir>) {
        if self.trailing_wildcards != 0 {
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
            hirs.insert(0, Hir::look(Look::Start));
        }
        if self.leading_wildcard {
            hirs.push(Hir::look(Look::End))
        }
    }
}

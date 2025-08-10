use regex_syntax::{
    hir::{Hir, HirKind},
    Error,
};

pub fn parse_and_fold_literal(
    pattern: &str,
) -> Result<(Hir, Vec<Box<[u8]>>), Error> {
    Ok(fold_literal(regex_syntax::parse(pattern)?))
}

pub fn parse_and_fold_literal_utf8(
    pattern: &str,
) -> Result<(Hir, Vec<String>), Error> {
    Ok(fold_literal_utf8(regex_syntax::parse(pattern)?))
}

/// Fold the first 256 literals into single byte literals.
pub fn fold_literal(hir: Hir) -> (Hir, Vec<Box<[u8]>>) {
    fold_literal_common(hir, Ok)
}

/// Fold the first 256 UTF-8 literals into single byte literals.
pub fn fold_literal_utf8(hir: Hir) -> (Hir, Vec<String>) {
    fold_literal_common(hir, |b| String::from_utf8(b.to_vec()).map_err(|_| b))
}

fn fold_literal_common<T>(
    hir: Hir,
    try_into: impl Fn(Box<[u8]>) -> Result<T, Box<[u8]>>,
) -> (Hir, Vec<T>) {
    fn fold_literal<T>(
        hir: Hir,
        literals: &mut Vec<T>,
        f: &impl Fn(Box<[u8]>) -> Result<T, Box<[u8]>>,
    ) -> Hir {
        match hir.kind() {
            HirKind::Empty | HirKind::Class(_) | HirKind::Look(_) => hir,
            HirKind::Literal(_) => {
                let i = literals.len();
                if i > u8::MAX as usize {
                    // Too many literals
                    return hir;
                }

                let literal = match hir.into_kind() {
                    HirKind::Literal(literal) => literal,
                    _ => unreachable!(),
                };
                match f(literal.0) {
                    Ok(literal) => {
                        literals.push(literal);
                        Hir::literal([i as u8])
                    }
                    Err(literal) => Hir::literal(literal),
                }
            }
            HirKind::Repetition(_) => {
                let mut repetition = match hir.into_kind() {
                    HirKind::Repetition(repetition) => repetition,
                    _ => unreachable!(),
                };
                repetition.sub =
                    fold_literal(*repetition.sub, literals, f).into();
                Hir::repetition(repetition)
            }
            HirKind::Capture(_) => {
                let mut capture = match hir.into_kind() {
                    HirKind::Capture(capture) => capture,
                    _ => unreachable!(),
                };
                capture.sub = fold_literal(*capture.sub, literals, f).into();
                Hir::capture(capture)
            }
            HirKind::Concat(_) => {
                let subs = match hir.into_kind() {
                    HirKind::Concat(subs) => subs,
                    _ => unreachable!(),
                }
                .into_iter()
                .map(|sub| fold_literal(sub, literals, f))
                .collect();
                Hir::concat(subs)
            }
            HirKind::Alternation(_) => {
                let subs = match hir.into_kind() {
                    HirKind::Alternation(subs) => subs,
                    _ => unreachable!(),
                }
                .into_iter()
                .map(|sub| fold_literal(sub, literals, f))
                .collect();
                Hir::alternation(subs)
            }
        }
    }
    let mut literals = Vec::new();
    (fold_literal(hir, &mut literals, &try_into), literals)
}

#[cfg(test)]
mod tests {
    use regex_syntax::{hir::Hir, parse};

    use super::*;

    #[test]
    fn fold_literal_test() {
        let (hir, literals) = parse_and_fold_literal_utf8("abc").unwrap();
        assert_eq!(hir, Hir::literal(*b"\x00"));
        assert_eq!(literals, vec!["abc".to_string()]);

        let (hir, literals) = parse_and_fold_literal_utf8("abc.*def").unwrap();
        assert_eq!(
            hir,
            Hir::concat(vec![
                Hir::literal(*b"\x00"),
                parse(".*").unwrap(),
                Hir::literal(*b"\x01")
            ])
        );
        assert_eq!(literals, vec!["abc".to_string(), "def".to_string()]);
    }
}

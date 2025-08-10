use regex_syntax::{
    hir::{Hir, HirKind},
    Error,
};

pub fn parse_and_fold_literal(
    pattern: &str,
) -> Result<(Hir, Vec<Box<[u8]>>), Error> {
    Ok(fold_literal(regex_syntax::parse(pattern)?))
}

pub fn parse_and_fold_literal_str(
    pattern: &str,
) -> Result<(Hir, Vec<String>), Error> {
    let (hir, literals) = parse_and_fold_literal(pattern)?;
    Ok((
        hir,
        literals
            .into_iter()
            .map(|b| String::from_utf8(b.to_vec()).unwrap())
            .collect(),
    ))
}

/// Fold the first 256 literals into single byte literals.
pub fn fold_literal(hir: Hir) -> (Hir, Vec<Box<[u8]>>) {
    fn fold_literal(hir: Hir, literals: &mut Vec<Box<[u8]>>) -> Hir {
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
                literals.push(literal.0);
                Hir::literal([i as u8])
            }
            HirKind::Repetition(_) => {
                let mut repetition = match hir.into_kind() {
                    HirKind::Repetition(repetition) => repetition,
                    _ => unreachable!(),
                };
                repetition.sub =
                    fold_literal(*repetition.sub, literals).into();
                Hir::repetition(repetition)
            }
            HirKind::Capture(_) => {
                let mut capture = match hir.into_kind() {
                    HirKind::Capture(capture) => capture,
                    _ => unreachable!(),
                };
                capture.sub = fold_literal(*capture.sub, literals).into();
                Hir::capture(capture)
            }
            HirKind::Concat(_) => {
                let subs = match hir.into_kind() {
                    HirKind::Concat(subs) => subs,
                    _ => unreachable!(),
                }
                .into_iter()
                .map(|sub| fold_literal(sub, literals))
                .collect();
                Hir::concat(subs)
            }
            HirKind::Alternation(_) => {
                let subs = match hir.into_kind() {
                    HirKind::Alternation(subs) => subs,
                    _ => unreachable!(),
                }
                .into_iter()
                .map(|sub| fold_literal(sub, literals))
                .collect();
                Hir::alternation(subs)
            }
        }
    }
    let mut literals = Vec::new();
    (fold_literal(hir, &mut literals), literals)
}

#[cfg(test)]
mod tests {
    use regex_syntax::{hir::Hir, parse};

    use super::*;

    #[test]
    fn fold_literal_test() {
        let (hir, literals) = parse_and_fold_literal_str("abc").unwrap();
        assert_eq!(hir, Hir::literal(*b"\x00"));
        assert_eq!(literals, vec!["abc".to_string()]);

        let (hir, literals) = parse_and_fold_literal_str("abc.*def").unwrap();
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

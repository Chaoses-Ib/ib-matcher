use itertools::Itertools;
use regex_syntax::hir::{Class, ClassBytes, ClassBytesRange, Hir, HirKind};

pub fn literal_to_ascii_case_insensitive(s: &[u8]) -> Hir {
    let mut hirs = Vec::with_capacity(s.len());
    for (is_ascii_alphabetic, group) in
        &s.iter().copied().chunk_by(u8::is_ascii_alphabetic)
    {
        if is_ascii_alphabetic {
            for c in group {
                let mut class = ClassBytes::new([ClassBytesRange::new(c, c)]);
                class.case_fold_simple();
                hirs.push(Hir::class(Class::Bytes(class)))
            }
        } else {
            // Even without `chunk_by()`, `Hir::concat()` will still optimize it to the same form.
            // But every `Hir::literal` (and `class`) needs a `Box`, so we best chunk it.
            hirs.push(Hir::literal(group.collect_vec().into_boxed_slice()))
        }
    }
    let hir = Hir::concat(hirs);
    #[cfg(test)]
    if let Ok(s) = std::str::from_utf8(s) {
        let hir2 = regex_syntax::ParserBuilder::new()
            .case_insensitive(true)
            .unicode(false)
            .utf8(false)
            .build()
            .parse(&regex_syntax::escape(s))
            .unwrap();
        assert_eq!(hir, hir2);
    }
    hir
}

pub fn hir_to_ascii_case_insensitive(hir: Hir) -> Hir {
    match hir.kind() {
        HirKind::Empty | HirKind::Look(_) => hir,
        HirKind::Literal(_) => {
            let literal = match hir.into_kind() {
                HirKind::Literal(literal) => literal,
                _ => unreachable!(),
            };
            literal_to_ascii_case_insensitive(&literal.0)
        }
        HirKind::Class(_) => {
            // TODO
            hir
        }
        HirKind::Repetition(_) => {
            let mut repetition = match hir.into_kind() {
                HirKind::Repetition(repetition) => repetition,
                _ => unreachable!(),
            };
            repetition.sub =
                hir_to_ascii_case_insensitive(*repetition.sub).into();
            Hir::repetition(repetition)
        }
        HirKind::Capture(_) => {
            let mut capture = match hir.into_kind() {
                HirKind::Capture(capture) => capture,
                _ => unreachable!(),
            };
            capture.sub = hir_to_ascii_case_insensitive(*capture.sub).into();
            Hir::capture(capture)
        }
        HirKind::Concat(_) => {
            let subs = match hir.into_kind() {
                HirKind::Concat(subs) => subs,
                _ => unreachable!(),
            }
            .into_iter()
            .map(hir_to_ascii_case_insensitive)
            .collect();
            Hir::concat(subs)
        }
        HirKind::Alternation(_) => {
            let subs = match hir.into_kind() {
                HirKind::Alternation(subs) => subs,
                _ => unreachable!(),
            }
            .into_iter()
            .map(hir_to_ascii_case_insensitive)
            .collect();
            Hir::alternation(subs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case() {
        let hir = literal_to_ascii_case_insensitive(b"++");
        println!("{:?}", hir);

        let hir = literal_to_ascii_case_insensitive(b"prog++ram");
        println!("{:?}", hir);

        let hir = hir_to_ascii_case_insensitive(Hir::literal(
            "prog++ram".as_bytes(),
        ));
        println!("{:?}", hir);
    }
}

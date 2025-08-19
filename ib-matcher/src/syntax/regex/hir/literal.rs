use regex_syntax::hir::Hir;

pub use regex_syntax::hir::literal::*;

pub fn extract_first_byte(hirs: &[Hir]) -> Option<u8> {
    let mut extractor = Extractor::new();
    extractor
        .kind(ExtractKind::Prefix)
        .limit_class(1)
        .limit_repeat(1)
        .limit_literal_len(1)
        .limit_total(2);

    let mut prefixes = Seq::empty();
    for hir in hirs {
        prefixes.union(&mut extractor.extract(hir));
    }
    #[cfg(test)]
    println!(
        "prefixes (len={:?}, exact={:?}) extracted: {:?}",
        prefixes.len(),
        prefixes.is_exact(),
        prefixes
    );

    prefixes
        .literals()
        .filter(|l| {
            // 0: empty hirs, >1: many hirs
            l.len() == 1
        })
        .and_then(|l| {
            let l = unsafe { l.get_unchecked(0) };
            // May be ""
            debug_assert!(l.as_bytes().len() <= 1);
            l.as_bytes().first().copied()
        })
}

#[cfg(test)]
mod tests {
    use regex_syntax::{hir::Look, parse};

    use super::*;

    #[test]
    fn extract_first_byte_test() {
        assert_eq!(extract_first_byte(&[]), None);
        assert_eq!(extract_first_byte(&[parse("").unwrap()]), None);
        assert_eq!(extract_first_byte(&[parse("a").unwrap()]), Some(b'a'));
        assert_eq!(extract_first_byte(&[parse("a|ab").unwrap()]), Some(b'a'));
        assert_eq!(
            extract_first_byte(&[parse("a|ab|abc|aki|azki|ahegao").unwrap()]),
            Some(b'a')
        );
        assert_eq!(
            extract_first_byte(&[parse("a{3}|ab|abc").unwrap()]),
            Some(b'a')
        );
        assert_eq!(extract_first_byte(&[parse("a|b").unwrap()]), None);
        assert_eq!(
            extract_first_byte(&[parse("(a|(ab))").unwrap()]),
            Some(b'a')
        );

        assert_eq!(
            extract_first_byte(&[Hir::concat(vec![
                Hir::look(Look::StartCRLF),
                Hir::literal("foo".as_bytes()),
                Hir::look(Look::EndCRLF),
            ])]),
            Some(b'f')
        );
        assert_eq!(
            extract_first_byte(&[
                Hir::concat(vec![
                    Hir::look(Look::StartCRLF),
                    Hir::literal("foo".as_bytes()),
                    Hir::look(Look::EndCRLF),
                ]),
                Hir::concat(vec![
                    Hir::look(Look::StartCRLF),
                    Hir::literal("bar".as_bytes()),
                    Hir::look(Look::EndCRLF),
                ])
            ]),
            None
        );
        assert_eq!(
            extract_first_byte(&[
                Hir::concat(vec![
                    Hir::look(Look::StartCRLF),
                    Hir::literal("foo".as_bytes()),
                    Hir::look(Look::EndCRLF),
                ]),
                Hir::concat(vec![
                    Hir::look(Look::StartCRLF),
                    Hir::literal("bar".as_bytes()),
                    Hir::look(Look::EndCRLF),
                ]),
                Hir::concat(vec![
                    Hir::look(Look::StartCRLF),
                    Hir::literal("far".as_bytes()),
                    Hir::look(Look::EndCRLF),
                ])
            ]),
            None
        );
        assert_eq!(
            extract_first_byte(&[
                Hir::concat(vec![
                    Hir::look(Look::StartCRLF),
                    Hir::literal("foo".as_bytes()),
                    Hir::look(Look::EndCRLF),
                ]),
                Hir::concat(vec![
                    Hir::look(Look::StartCRLF),
                    Hir::literal("far".as_bytes()),
                    Hir::look(Look::EndCRLF),
                ]),
                Hir::concat(vec![
                    Hir::look(Look::StartCRLF),
                    Hir::literal("far".as_bytes()),
                    Hir::look(Look::EndCRLF),
                ])
            ]),
            Some(b'f')
        );
    }
}

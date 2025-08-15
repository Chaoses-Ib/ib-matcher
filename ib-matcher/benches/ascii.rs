//! cargo bench --bench ascii --features perf-plain-regex
use std::hint::black_box;

use aho_corasick::automaton::Automaton;
use criterion::{criterion_group, criterion_main, Criterion};
use ib_matcher::{
    matcher::{IbMatcher, PinyinMatchConfig},
    pinyin::PinyinNotation,
};

pub fn criterion_benchmark(c: &mut Criterion) {
    {
        // For one pattern, if ascii_case_insensitive is false, the prefilter is memmem; otherwise only the first start byte and one rare byte will be searched.
        let dfa = aho_corasick::dfa::Builder::new()
            .ascii_case_insensitive(true)
            .prefilter(true)
            .build(&["pysseve"])
            .unwrap();
        let prefilter = dfa.prefilter().unwrap();
        let candidate = prefilter.find_in(
            "pyssEverything".as_bytes(),
            aho_corasick::Span { start: 0, end: 14 },
        );
        dbg!(candidate);
        assert!(candidate.into_option().is_some());
        c.bench_function("find_ascii_ac_prefilter_only", |b| {
            b.iter(|| {
                prefilter
                    .find_in(
                        black_box("pyssEverything".as_bytes()),
                        aho_corasick::Span { start: 0, end: 14 },
                    )
                    .into_option()
                    .is_some()
            })
        });
        c.bench_function("find_ascii_20_ac_prefilter_only", |b| {
            b.iter(|| {
                prefilter
                    .find_in(
                        black_box("12345678901234567890pyssEverything".as_bytes()),
                        aho_corasick::Span { start: 0, end: 14 },
                    )
                    .into_option()
                    .is_some()
            })
        });
    }

    {
        let ac = daachorse::DoubleArrayAhoCorasick::<u32>::new(["pysseve"]).unwrap();
        assert!(ac.find_iter("pysseverything").next().is_some());
        c.bench_function("find_ascii_daachorse", |b| {
            b.iter(|| ac.find_iter(black_box("pyssEverything")).next())
        });

        c.bench_function("find_ascii_20_daachorse", |b| {
            b.iter(|| {
                ac.find_iter(black_box("12345678901234567890pyssEverything"))
                    .next()
            })
        });
    }
    {
        let ac = daachorse::CharwiseDoubleArrayAhoCorasick::<u32>::new(["pysseve"]).unwrap();
        assert!(ac.find_iter("pysseverything").next().is_some());
        c.bench_function("find_ascii_daachorse_charwise", |b| {
            b.iter(|| ac.find_iter(black_box("pyssEverything")).next())
        });

        c.bench_function("find_ascii_20_daachorse_charwise", |b| {
            b.iter(|| {
                ac.find_iter(black_box("12345678901234567890pyssEverything"))
                    .next()
            })
        });
    }

    {
        let dfa = aho_corasick::dfa::Builder::new()
            .ascii_case_insensitive(true)
            .prefilter(true)
            .build(&["pysseve"])
            .unwrap();
        assert!(dfa
            .try_find(&aho_corasick::Input::new("pyssEverything"))
            .unwrap()
            .is_some());
        c.bench_function("find_ascii_ac_dfa_prefilter", |b| {
            b.iter(|| {
                dfa.try_find(&aho_corasick::Input::new(black_box("pyssEverything")))
                    .unwrap()
            })
        });

        c.bench_function("find_ascii_20_ac_dfa_prefilter", |b| {
            b.iter(|| {
                dfa.try_find(&aho_corasick::Input::new(black_box(
                    "12345678901234567890pyssEverything",
                )))
                .unwrap()
            })
        });

        c.bench_function("find_ascii_fail_ac_dfa_prefilter", |b| {
            b.iter(|| {
                dfa.try_find(&aho_corasick::Input::new(black_box(
                    "12345678901234567890pyssEverythin",
                )))
                .unwrap()
            })
        });
    }
    {
        let dfa = aho_corasick::dfa::Builder::new()
            .ascii_case_insensitive(true)
            .prefilter(false)
            .build(&["pysseve"])
            .unwrap();
        assert!(dfa
            .try_find(&aho_corasick::Input::new("pyssEverything"))
            .unwrap()
            .is_some());
        c.bench_function("find_ascii_ac_dfa", |b| {
            b.iter(|| {
                dfa.try_find(&aho_corasick::Input::new(black_box("pyssEverything")))
                    .unwrap()
            })
        });

        c.bench_function("find_ascii_20_ac_dfa", |b| {
            b.iter(|| {
                dfa.try_find(&aho_corasick::Input::new(black_box(
                    "12345678901234567890pyssEverything",
                )))
                .unwrap()
            })
        });

        c.bench_function("find_ascii_fail_ac_dfa", |b| {
            b.iter(|| {
                dfa.try_find(&aho_corasick::Input::new(black_box(
                    "12345678901234567890pyssEverythin",
                )))
                .unwrap()
            })
        });
    }

    {
        assert!("pysseverything".find("pysseve").is_some());
        c.bench_function("find_ascii_std", |b| {
            b.iter(|| black_box("pysseverything").find("pysseve"))
        });
    }

    {
        let ac = aho_corasick::AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(&["pysseve"])
            .unwrap();
        assert!(ac.find("pyssEverything").is_some());
        c.bench_function("find_ascii_ac", |b| {
            b.iter(|| ac.find(black_box("pyssEverything")))
        });
    }

    {
        let re = regex_automata::dfa::regex::Regex::builder()
            .syntax(
                regex_automata::util::syntax::Config::new()
                    .utf8(false)
                    .unicode(false)
                    .case_insensitive(true),
            )
            .build(r"pysseve")
            .unwrap();
        assert!(re.find("pyssEverything").is_some());
        c.bench_function("find_ascii_regex_dfa", |b| {
            b.iter(|| re.find(black_box("pyssEverything")))
        });
    }

    {
        let regex = regex::RegexBuilder::new("pysseve")
            .unicode(false)
            .case_insensitive(true)
            .build()
            .unwrap();
        assert!(regex.find("pyssEverything").is_some());
        c.bench_function("find_ascii_regex", |b| {
            b.iter(|| regex.find(black_box("pyssEverything")))
        });

        let regex = regex::bytes::RegexBuilder::new("pysseve")
            .unicode(false)
            .case_insensitive(true)
            .build()
            .unwrap();
        assert!(regex.find(b"pyssEverything").is_some());
        c.bench_function("find_ascii_regex_bytes", |b| {
            b.iter(|| regex.find(black_box(b"pyssEverything")))
        });

        let regex = regex::bytes::RegexBuilder::new("\\x70\\x79\\x73\\x73\\x65\\x76\\x65")
            .unicode(false)
            .case_insensitive(true)
            .build()
            .unwrap();
        assert!(regex.find(b"pyssEverything").is_some());
        c.bench_function("find_ascii_regex_bytes_x", |b| {
            b.iter(|| regex.find(black_box(b"pyssEverything")))
        });
    }

    {
        let matcher = IbMatcher::builder("pysseve")
            .pinyin(PinyinMatchConfig::notations(
                PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
            ))
            .build();
        assert!(matcher.find("pyssEverything").is_some());
        c.bench_function("find_ascii", |b| {
            b.iter(|| matcher.find(black_box("pyssEverything")))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

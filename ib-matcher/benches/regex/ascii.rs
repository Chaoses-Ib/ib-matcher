use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use ib_matcher::{
    matcher::{IbMatcher, MatchConfig, PinyinMatchConfig},
    regex::{cp, lita},
};
use regex_automata::{
    dfa::dense,
    util::{prefilter::Prefilter, syntax},
    MatchKind,
};
use regex_syntax::parse;

pub fn criterion_benchmark(c: &mut Criterion) {
    let ascii_20 = "12345678901234567890pyEverythingss";

    {
        let re = IbMatcher::builder("ss").build();
        assert!(re.find(ascii_20).is_some());
        c.bench_function("ib", |b| b.iter(|| re.find(black_box(ascii_20))));
    }

    {
        let re = regex_automata::dfa::regex::Regex::builder()
            .syntax(syntax::Config::new().utf8(false))
            .build(r"(?s-u)py[^\\]*ss")
            .unwrap();
        assert!(re.find(ascii_20).is_some());
        c.bench_function("re_dfa_dense", |b| b.iter(|| re.find(black_box(ascii_20))));

        // TODO: Slower?
        // memmem is not used? Or too short?
        let re = regex_automata::dfa::regex::Regex::builder()
            .syntax(syntax::Config::new().utf8(false))
            .dense(
                dense::Config::new().prefilter(Prefilter::new(MatchKind::LeftmostFirst, &["py"])),
            )
            .build(r"(?s-u)py[^\\]*ss")
            .unwrap();
        assert!(re.find(ascii_20).is_some());
        c.bench_function("re_dfa_dense_prefilter", |b| {
            b.iter(|| re.find(black_box(ascii_20)))
        });

        let re = regex_automata::dfa::regex::Regex::builder()
            .syntax(syntax::Config::new().utf8(false))
            .dense(dense::Config::new().prefilter(Prefilter::from_hir_prefix(
                MatchKind::LeftmostFirst,
                &parse(r"(?s-u)py").unwrap(),
            )))
            .build(r"(?s-u)py[^\\]*ss")
            .unwrap();
        assert!(re.find(ascii_20).is_some());
        c.bench_function("re_dfa_dense_prefilter_parse", |b| {
            b.iter(|| re.find(black_box(ascii_20)))
        });
    }

    {
        let re = lita::Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(PinyinMatchConfig::default())
                .build())
            .syntax(syntax::Config::new().utf8(false))
            .build(r"(?s-u)py[^\\]*ss")
            .unwrap();
        assert!(re.find(ascii_20).is_some());
        c.bench_function("re_lita", |b| b.iter(|| re.find(black_box(ascii_20))));
    }

    {
        let re = regex_automata::hybrid::regex::Regex::builder()
            .syntax(syntax::Config::new().utf8(false))
            .build(r"(?s-u)py[^\\]*ss")
            .unwrap();
        let mut cache = re.create_cache();
        assert!(re.find(&mut cache, ascii_20).is_some());
        c.bench_function("re_hybrid", |b| {
            b.iter(|| re.find(&mut cache, black_box(ascii_20)))
        });
    }

    {
        let re = regex_automata::meta::Regex::builder()
            .syntax(syntax::Config::new().utf8(false))
            .build(r"(?s-u)py[^\\]*ss")
            .unwrap();
        assert!(re.find(ascii_20).is_some());
        c.bench_function("re_meta", |b| b.iter(|| re.find(black_box(ascii_20))));
    }

    {
        let re = regex_automata::dfa::regex::Regex::builder()
            .syntax(syntax::Config::new().utf8(false))
            .build_sparse(r"(?s-u)py[^\\]*ss")
            .unwrap();
        assert!(re.find(ascii_20).is_some());
        c.bench_function("re_dfa_sparse", |b| b.iter(|| re.find(black_box(ascii_20))));
    }

    {
        // let re = regex_automata::dfa::onepass::Builder::new()
        //     .syntax(syntax::Config::new().utf8(false))
        //     .build(r"(?s-u)py[^\\]*ss")
        //     .unwrap();
        // let mut cache = re.create_cache();
        // assert!(re.find(&mut cache, ascii_20).is_some());
        // c.bench_function("re_dfa_onepass", |b| {
        //     b.iter(|| re.find(&mut cache, black_box(ascii_20)))
        // });
    }

    {
        let re = regex_automata::nfa::thompson::backtrack::Builder::new()
            .syntax(syntax::Config::new().utf8(false))
            .build(r"(?s-u)py[^\\]*ss")
            .unwrap();
        let mut cache = re.create_cache();
        assert!(re.try_find(&mut cache, ascii_20).unwrap().is_some());
        c.bench_function("re_backtrack", |b| {
            b.iter(|| re.try_find(&mut cache, black_box(ascii_20)))
        });
    }

    {
        let re = ib_matcher::regex::nfa::backtrack::Builder::new()
            .syntax(syntax::Config::new().utf8(false))
            .build(r"(?s-u)py[^\\]*ss")
            .unwrap();
        let mut cache = re.create_cache();
        assert!(re.try_find(&mut cache, ascii_20).unwrap().is_some());
        c.bench_function("re_backtrack_ib", |b| {
            b.iter(|| re.try_find(&mut cache, black_box(ascii_20)))
        });
    }

    {
        let re = cp::Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(PinyinMatchConfig::default())
                .build())
            .syntax(syntax::Config::new().utf8(false))
            .build(r"(?s-u)py[^\\]*ss")
            .unwrap();
        let mut cache = re.create_cache();
        assert!(re.try_find(&mut cache, ascii_20).unwrap().is_some());
        c.bench_function("re_backtrack_ib_cp", |b| {
            b.iter(|| re.try_find(&mut cache, black_box(ascii_20)))
        });

        let re = cp::Regex::builder()
            .ib(MatchConfig::builder()
                .pinyin(PinyinMatchConfig::default())
                .build())
            .syntax(syntax::Config::new().utf8(false))
            .build(r"(?s-u)py[^\\]*ss")
            .unwrap();
        assert!(re.find(ascii_20).is_some());
        c.bench_function("re_backtrack_ib_cp_pool", |b| {
            b.iter(|| re.find(black_box(ascii_20)))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

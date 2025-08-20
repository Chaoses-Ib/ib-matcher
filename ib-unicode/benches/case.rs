use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use ib_unicode::case::CharCaseExt;

pub fn criterion_benchmark(c: &mut Criterion) {
    {
        assert_eq!('A'.to_mono_lowercase(), 'a');
        c.bench_function("mono_lowercase/ascii_hit", |b| {
            b.iter(|| black_box('A').to_mono_lowercase())
        });

        assert_eq!('!'.to_mono_lowercase(), '!');
        c.bench_function("mono_lowercase/ascii_miss", |b| {
            b.iter(|| black_box('!').to_mono_lowercase())
        });

        assert_eq!('Σ'.to_mono_lowercase(), 'σ');
        c.bench_function("mono_lowercase/uni_hit", |b| {
            b.iter(|| black_box('Σ').to_mono_lowercase())
        });

        assert_eq!('う'.to_mono_lowercase(), 'う');
        c.bench_function("mono_lowercase/uni_miss", |b| {
            b.iter(|| black_box('う').to_mono_lowercase())
        });
    }
    {
        assert_eq!('A'.to_simple_fold_case_map(), 'a');
        c.bench_function("simple_fold_fast/ascii_hit", |b| {
            b.iter(|| black_box('A').to_simple_fold_case_map())
        });

        assert_eq!('!'.to_simple_fold_case_map(), '!');
        c.bench_function("simple_fold_fast/ascii_miss", |b| {
            b.iter(|| black_box('!').to_simple_fold_case_map())
        });

        assert_eq!('Σ'.to_simple_fold_case_map(), 'σ');
        c.bench_function("simple_fold_fast/uni_hit", |b| {
            b.iter(|| black_box('Σ').to_simple_fold_case_map())
        });

        assert_eq!('う'.to_simple_fold_case_map(), 'う');
        c.bench_function("simple_fold_fast/uni_miss", |b| {
            b.iter(|| black_box('う').to_simple_fold_case_map())
        });
    }
    {
        assert_eq!('A'.to_simple_fold_case_unicase(), 'a');
        c.bench_function("simple_fold/ascii_hit", |b| {
            b.iter(|| black_box('A').to_simple_fold_case_unicase())
        });

        assert_eq!('!'.to_simple_fold_case_unicase(), '!');
        c.bench_function("simple_fold/ascii_miss", |b| {
            b.iter(|| black_box('!').to_simple_fold_case_unicase())
        });

        assert_eq!('Σ'.to_simple_fold_case_unicase(), 'σ');
        c.bench_function("simple_fold/uni_hit", |b| {
            b.iter(|| black_box('Σ').to_simple_fold_case_unicase())
        });

        assert_eq!('う'.to_simple_fold_case_unicase(), 'う');
        c.bench_function("simple_fold/uni_miss", |b| {
            b.iter(|| black_box('う').to_simple_fold_case_unicase())
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

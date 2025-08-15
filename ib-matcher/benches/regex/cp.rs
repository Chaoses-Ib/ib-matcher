use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use ib_matcher::{
    matcher::{MatchConfig, PinyinMatchConfig},
    pinyin::PinyinNotation,
    regex::cp::Regex,
};

pub fn criterion_benchmark(c: &mut Criterion) {
    let matcher = Regex::builder()
        .ib(MatchConfig::builder()
            .pinyin(PinyinMatchConfig::notations(
                PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
            ))
            .build())
        .build("pysseve")
        .unwrap();
    let analyzed = Regex::builder()
        .ib(MatchConfig::builder()
            .pinyin(PinyinMatchConfig::notations(
                PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
            ))
            .analyze(true)
            .build())
        .build("pysseve")
        .unwrap();

    assert!(matcher.find("pys").is_none());
    c.bench_function("find_ascii_too_short", |b| {
        b.iter(|| matcher.find(black_box("pys")))
    });

    assert!(matcher.find("拼").is_none());
    c.bench_function("find_too_short_analyse", |b| {
        b.iter(|| analyzed.find(black_box("拼")))
    });
    c.bench_function("find_too_short", |b| {
        b.iter(|| matcher.find(black_box("拼")))
    });

    assert!(matcher.is_match("pyssEverything"));
    c.bench_function("is_match_ascii", |b| {
        b.iter(|| matcher.is_match(black_box("pyssEverything")))
    });

    assert!(matcher.find("pyssEverything").is_some());
    c.bench_function("find_ascii", |b| {
        b.iter(|| matcher.find(black_box("pyssEverything")))
    });

    assert!(matcher.is_match("拼音搜索Everything"));
    c.bench_function("is_match", |b| {
        b.iter(|| matcher.is_match(black_box("拼音搜索Everything")))
    });

    assert!(matcher.find("拼音搜索Everything").is_some());
    c.bench_function("find", |b| {
        b.iter(|| matcher.find(black_box("拼音搜索Everything")))
    });

    assert!(matcher.find("あなたは誰拼音搜索Everything").is_some());
    c.bench_function("find_5", |b| {
        b.iter(|| matcher.find(black_box("あなたは誰拼音搜索Everything")))
    });

    assert!(matcher.find("あなたは誰拼音搜索Evvrything").is_none());
    c.bench_function("find_5_miss_analyze", |b| {
        b.iter(|| analyzed.find(black_box("あなたは誰拼音搜索Evvrything")))
    });
    c.bench_function("find_5_miss", |b| {
        b.iter(|| matcher.find(black_box("あなたは誰拼音搜索Evvrything")))
    });

    let ascii_25 = "12345678901234567890あなたは誰拼音搜索Everything";
    assert!(analyzed.find(ascii_25).is_some());
    c.bench_function("find_ascii_25_analyse", |b| {
        b.iter(|| analyzed.find(black_box(ascii_25)))
    });
    assert!(matcher.find(ascii_25).is_some());
    c.bench_function("find_ascii_25", |b| {
        b.iter(|| matcher.find(black_box(ascii_25)))
    });

    c.bench_function("build", |b| {
        b.iter(|| {
            Regex::builder()
                .ib(MatchConfig::builder()
                    .pinyin(PinyinMatchConfig::notations(
                        PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
                    ))
                    .build())
                .build("pysseve")
                .unwrap()
        })
    });
    c.bench_function("build_analyze", |b| {
        b.iter(|| {
            Regex::builder()
                .ib(MatchConfig::builder()
                    .pinyin(PinyinMatchConfig::notations(
                        PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
                    ))
                    .analyze(true)
                    .build())
                .build("pysseve")
                .unwrap()
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use ib_romaji::HepburnRomanizer;

pub fn criterion_benchmark(c: &mut Criterion) {
    // 8ns
    assert!(HepburnRomanizer::is_romaji_kana_boundary("tokidoki", 8));
    c.bench_function("boundary_8_hit", |b| {
        b.iter(|| HepburnRomanizer::is_romaji_kana_boundary(black_box("tokidoki"), black_box(8)))
    });

    // 66ns
    assert!(HepburnRomanizer::is_romaji_kana_boundary(
        "shintaihappukorewofuboniukuaetekishousezaruhakounohajimenari",
        60
    ));
    c.bench_function("boundary_60_hit", |b| {
        b.iter(|| {
            HepburnRomanizer::is_romaji_kana_boundary(
                black_box("shintaihappukorewofuboniukuaetekishousezaruhakounohajimenari"),
                black_box(60),
            )
        })
    });

    // 68ns
    assert!(HepburnRomanizer::is_romaji_kana_boundary(
        "shintaihappukorewofuboniukuaetekishousezaruhakounohajimenari",
        60
    ));
    c.bench_function("boundary_60_miss", |b| {
        b.iter(|| {
            HepburnRomanizer::is_romaji_kana_boundary(
                black_box("shintaihappukorewofuboniukuaetekishousezaruhakounohajimenarx"),
                black_box(60),
            )
        })
    });

    // 84ns
    c.bench_function("boundary_mix68_hit", |b| {
        b.iter(|| {
            (
                HepburnRomanizer::is_romaji_kana_boundary(black_box("tokidoki"), black_box(8)),
                HepburnRomanizer::is_romaji_kana_boundary(
                    black_box("shintaihappukorewofuboniukuaetekishousezaruhakounohajimenari"),
                    black_box(60),
                ),
            )
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

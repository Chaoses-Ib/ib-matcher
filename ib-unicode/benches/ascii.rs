use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use ib_unicode::ascii::{find_byte2, find_non_ascii_byte};

pub fn criterion_benchmark(c: &mut Criterion) {
    assert!(find_non_ascii_byte("12345678901234567890ｗ".as_bytes()).is_some());
    c.bench_function("find_non_ascii_byte", |b| {
        b.iter(|| find_non_ascii_byte(black_box("12345678901234567890ｗ".as_bytes())))
    });

    assert!(find_byte2(b"12345678901234567890a", b'a', b'A').is_some());
    c.bench_function("find_byte2", |b| {
        b.iter(|| find_byte2(black_box(b"12345678901234567890a"), b'a', b'A'))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

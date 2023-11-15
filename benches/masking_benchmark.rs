use criterion::{black_box, criterion_group, criterion_main, Criterion};

use websocket_sans_io::apply_mask;

fn small_buffer(c: &mut Criterion) {
    let mask = *b"\x23\x34\x55\x00";
    let mut buffer = [0u8; 40];
    c.bench_function("mask phase=0", |b| b.iter(|| {
        apply_mask(black_box(mask), black_box(&mut buffer[..]), 0);
        black_box(&mut buffer);
    }));
    c.bench_function("mask phase=1", |b| b.iter(|| {
        apply_mask(black_box(mask), black_box(&mut buffer[..]), 1);
        black_box(&mut buffer);
    }));
}

fn large_buffer(c: &mut Criterion) {
    let mask = *b"\x23\x34\x55\x00";
    let mut buffer = vec![0u8; 65536];
    c.bench_function("mask large phase=0", |b| b.iter(|| {
        apply_mask(black_box(mask), black_box(&mut buffer[..]), 0);
        black_box(&mut buffer);
    }));
    c.bench_function("mask large phase=1", |b| b.iter(|| {
        apply_mask(black_box(mask), black_box(&mut buffer[..]), 1);
        black_box(&mut buffer);
    }));
}

criterion_group!{
    name = benches;
    config = Criterion::default().significance_level(0.02).sample_size(2000);
    targets = small_buffer, large_buffer
}
criterion_main!(benches);

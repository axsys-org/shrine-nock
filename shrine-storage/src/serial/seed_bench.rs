use criterion::{black_box, criterion_group, criterion_main, Criterion};
use seed::Seed;

fn bench_word_insertion(c: &mut Criterion) {
    c.bench_function("insert 1000 words", |b| {
        b.iter(|| {
            let mut seed = Seed::new();
            for i in 0..1000u64 {
                black_box(seed.word(i));
            }
        })
    });
}

fn bench_dedup(c: &mut Criterion) {
    c.bench_function("insert 1000 duplicate words", |b| {
        b.iter(|| {
            let mut seed = Seed::new();
            for _ in 0..1000 {
                black_box(seed.word(42));
            }
        })
    });
}

fn bench_cons(c: &mut Criterion) {
    c.bench_function("build tree of 1000 cons", |b| {
        b.iter(|| {
            let mut seed = Seed::new();
            let zero = seed.word(0);
            let mut current = zero;
            for i in 1..1000u64 {
                let n = seed.word(i);
                current = seed.cons(n, current);
            }
            black_box(current)
        })
    });
}

fn bench_serialize(c: &mut Criterion) {
    let mut seed = Seed::new();
    let zero = seed.word(0);
    let mut current = zero;
    for i in 1..100u64 {
        let n = seed.word(i);
        current = seed.cons(n, current);
    }
    seed.done().unwrap();
    
    let size = seed.size();
    let mut buf = vec![0u8; size];

    c.bench_function("serialize tree", |b| {
        b.iter(|| {
            black_box(seed.save(&mut buf));
        })
    });
}

fn bench_deserialize(c: &mut Criterion) {
    let mut seed = Seed::new();
    let zero = seed.word(0);
    let mut current = zero;
    for i in 1..100u64 {
        let n = seed.word(i);
        current = seed.cons(n, current);
    }
    seed.done().unwrap();
    
    let size = seed.size();
    let mut buf = vec![0u8; size];
    seed.save(&mut buf);

    c.bench_function("deserialize tree", |b| {
        b.iter(|| {
            let mut seed2 = Seed::new();
            black_box(seed2.load(&buf).unwrap());
        })
    });
}

criterion_group!(
    benches,
    bench_word_insertion,
    bench_dedup,
    bench_cons,
    bench_serialize,
    bench_deserialize
);
criterion_main!(benches);

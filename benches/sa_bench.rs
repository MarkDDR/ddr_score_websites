use criterion::{black_box, criterion_group, criterion_main, Criterion};
use score_websites::website_backends::skill_attack::{cut_webpage, get_scores_inner};

pub fn sa_parse_benchmark(c: &mut Criterion) {
    let webpage = std::fs::read_to_string("skill_attack.html").unwrap();
    c.bench_function("skill attack score parse", |b| {
        b.iter(|| {
            let webpage = cut_webpage(black_box(&webpage)).unwrap();
            get_scores_inner(webpage).unwrap()
        })
    });
}

criterion_group!(benches, sa_parse_benchmark);
criterion_main!(benches);

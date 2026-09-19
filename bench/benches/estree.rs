mod functions;

use criterion::{Criterion, criterion_group, criterion_main};

fn bench_estree(criterion: &mut Criterion) {
    let fixtures: [functions::fixtures::Fixture; 2] =
        functions::fixtures::fixtures();

    // rust -> js
    functions::codec_serialize::bench(criterion, &fixtures);

    // js -> rust
    functions::json_parse::bench(criterion, &fixtures);
    functions::ast_read::bench(criterion, &fixtures);
    functions::codec_json_to_program::bench(criterion, &fixtures);
}

criterion_group!(benches, bench_estree);
criterion_main!(benches);

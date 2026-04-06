use criterion::{Criterion, criterion_group, criterion_main};

mod local_hot_paths;

fn bench_all(c: &mut Criterion) {
    local_hot_paths::bench_json_path_extraction(c);
    local_hot_paths::bench_pipeline_extraction_loop(c);
    local_hot_paths::bench_live_check_matching(c);
    local_hot_paths::bench_title_cleaning(c);
    local_hot_paths::bench_template_rendering(c);
}

criterion_group!(benches, bench_all);
criterion_main!(benches);

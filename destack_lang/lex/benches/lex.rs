use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use destack_lang_lex::tokenize;
use destack_std_fs::glob;
use std::fs;

fn bench_tokenize(c: &mut Criterion) {
    // global all files in workspace root
    const WORKSPACE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
    let ds_files = glob::glob(&format!("{WORKSPACE_ROOT}/**/*.ds"));

    let mut group = c.benchmark_group("lexer");
    for path in ds_files
        .into_iter()
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("ds"))
    {
        let label = path.to_string_lossy().to_string();
        let content = fs::read_to_string(&path).unwrap_or_default();
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("tokenize", label),
            content.as_str(),
            |b, input| {
                b.iter(|| {
                    let mut num_tokens = 0u32;
                    for _tok in tokenize(input) {
                        num_tokens += 1;
                    }
                    black_box(num_tokens);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_tokenize);
criterion_main!(benches);

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use destack_lang_lex::tokenize;
use destack_std_fs::glob;
use std::fs;
use std::path::PathBuf;

fn bench_tokenize(c: &mut Criterion) {
    // find workspace root by walking up until we find a known repo marker
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("version.txt").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();
    let workspace_root = workspace_root_path.to_string_lossy().into_owned();

    // glob all .ds files under the workspace root
    let ds_files = glob::glob(&format!("{workspace_root}/**/*.ds"));

    // concatenate all contents into a single big string
    let mut ds_str: String = String::new();
    for path in ds_files.iter() {
        assert!(
            path.extension().is_some() && path.extension().unwrap() == "ds",
            "path does not end with .ds: {path:?}"
        );
        let content = fs::read_to_string(path).unwrap_or_default();
        ds_str.push_str(&content);
    }

    // single benchmark over the whole workspace content
    let mut group = c.benchmark_group("lexer");
    let line_count = ds_str.lines().count() as u64;
    group.throughput(Throughput::Elements(line_count));
    group.bench_with_input(BenchmarkId::new("tokenize", "all"), &ds_str, |b, input| {
        b.iter(|| {
            let mut num_tokens = 0u32;
            for _tok in tokenize(input) {
                num_tokens += 1;
            }
            black_box(num_tokens);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_tokenize);
criterion_main!(benches);

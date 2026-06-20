use destack_repository::TraceSnapshot;

use super::{
    Cell, TestSession, TextTable, TraceCounts, TraceTable, artifact_counter, millis, stage_micros,
    time_micros,
};

/// One generated module graph.
#[derive(Debug, Clone, Copy)]
struct ModuleGraph {
    /// The number of generated modules.
    modules: usize,
    /// The number of modules in each strongly connected component.
    component_size: usize,
}

impl ModuleGraph {
    /// Return the generated file path edited by this graph.
    fn edit_path(self) -> String {
        let module = self.edited_module();

        format!("src/module-{module}.ds")
    }

    /// Return the module edited after the cold check.
    fn edited_module(self) -> usize {
        if self.component_size > 1 {
            self.component_size / 2
        } else {
            self.modules / 2
        }
    }
}

/// One measured check trace row.
#[derive(Debug, Clone, Copy)]
struct CheckMeasurement {
    /// Total wall time in microseconds.
    total_micros: u64,
    /// Attempt counts grouped by outcome.
    counts: TraceCounts,
    /// Modules whose imports were reread by component graph.
    changed_modules: Option<u64>,
    /// Total graph stage busy time in microseconds.
    graph_micros: u64,
    /// Total check stage busy time in microseconds.
    check_micros: u64,
    /// Time spent enumerating modules in microseconds.
    modules_micros: u64,
    /// Time spent reading import edges in microseconds.
    edges_micros: u64,
    /// Time spent partitioning SCCs in microseconds.
    scc_micros: u64,
    /// Time spent deriving component dependencies in microseconds.
    condensation_micros: u64,
}

impl CheckMeasurement {
    /// Build one measurement from a trace snapshot.
    fn from_trace(trace: &TraceSnapshot) -> Self {
        Self {
            total_micros: trace.total_micros,
            counts: TraceCounts::from_trace(trace),
            changed_modules: artifact_counter(trace, "component.graph", "changed_modules"),
            graph_micros: stage_micros(trace, "graph"),
            check_micros: stage_micros(trace, "check"),
            modules_micros: time_micros(trace, "modules"),
            edges_micros: time_micros(trace, "edges"),
            scc_micros: time_micros(trace, "scc"),
            condensation_micros: time_micros(trace, "condensation"),
        }
    }
}

/// Build one generated source tree fixture.
fn generated_source_tree(count: usize) -> Vec<(String, String)> {
    let mut files = vec![(
        "destack.json".to_string(),
        r#"{
  "name": "@test/app"
}
"#
        .to_string(),
    )];

    for index in 0..count {
        files.push((
            format!("src/generated/file-{index}.ds"),
            format!(
                r#"export const value{index} = {index};
"#
            ),
        ));
    }

    files
}

/// Build one generated module graph fixture.
fn generated_module_graph(graph: ModuleGraph) -> Vec<(String, String)> {
    let mut files = vec![(
        "destack.json".to_string(),
        r#"{
  "name": "@test/app"
}
"#
        .to_string(),
    )];

    for module in 0..graph.modules {
        files.push((
            format!("src/module-{module}.ds"),
            generated_module_source(graph, module, 1),
        ));
    }

    files
}

/// Build one generated source module.
fn generated_module_source(graph: ModuleGraph, module: usize, value: usize) -> String {
    let mut source = String::new();
    let group_start = module / graph.component_size * graph.component_size;
    let group_end = (group_start + graph.component_size).min(graph.modules);

    // import the next module to form one SCC per group
    if graph.component_size > 1 {
        let next = if module + 1 < group_end {
            module + 1
        } else {
            group_start
        };
        source.push_str(&format!(
            "import {{ value{next} }} from \"./module-{next}\";\n\n"
        ));
    }

    // export one changed value without changing imports
    source.push_str(&format!("export const value{module} = {value};\n"));

    source
}

/// Build the check scaling report table.
fn check_scaling_table(title: &str) -> TextTable {
    TextTable::new().title(title).color().row(vec![
        Cell::bold("modules"),
        Cell::bold("component"),
        Cell::bold("cold ms"),
        Cell::bold("cold artifacts"),
        Cell::bold("cold built"),
        Cell::bold("cold parked"),
        Cell::bold("edit ms"),
        Cell::bold("edit artifacts"),
        Cell::bold("edit built"),
        Cell::bold("edit parked"),
        Cell::bold("changed"),
        Cell::bold("graph ms"),
        Cell::bold("check ms"),
        Cell::bold("modules ms"),
        Cell::bold("edges ms"),
        Cell::bold("scc ms"),
        Cell::bold("condensation ms"),
    ])
}

#[test]
fn test_measure_check_after_single_module_edit() {
    let graphs = [
        ModuleGraph {
            modules: 10,
            component_size: 1,
        },
        ModuleGraph {
            modules: 10,
            component_size: 10,
        },
        ModuleGraph {
            modules: 100,
            component_size: 1,
        },
        ModuleGraph {
            modules: 100,
            component_size: 10,
        },
        ModuleGraph {
            modules: 100,
            component_size: 100,
        },
        ModuleGraph {
            modules: 1000,
            component_size: 1,
        },
        ModuleGraph {
            modules: 1000,
            component_size: 10,
        },
        ModuleGraph {
            modules: 1000,
            component_size: 100,
        },
    ];

    let mut table = check_scaling_table("check scaling");
    let mut detailed_traces = Vec::new();
    let worker_count = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);

    for graph in graphs {
        println!(
            "measuring {} modules with component size {}",
            graph.modules, graph.component_size
        );

        let files = generated_module_graph(graph);
        let files = files
            .iter()
            .map(|(path, content)| (path.as_str(), content.as_str()))
            .collect::<Vec<_>>();
        let test = TestSession::open_with_workers(&files, worker_count).unwrap();
        let edit_path = graph.edit_path();

        let (_cold, cold_trace) = test.check("src/module-0.ds", "js");
        let cold = CheckMeasurement::from_trace(&cold_trace);

        let edited_source = generated_module_source(graph, graph.edited_module(), 2);
        test.edit_text(&edit_path, &edited_source);

        let (_edited, edited_trace) = test.check("src/module-0.ds", "js");
        let edited = CheckMeasurement::from_trace(&edited_trace);

        assert_eq!(edited.changed_modules, Some(1));
        assert_eq!(edited.counts.failed, 0);

        table = table.row(vec![
            Cell::new(graph.modules.to_string()),
            Cell::new(graph.component_size.to_string()),
            Cell::new(format!("{:.3}", millis(cold.total_micros))),
            Cell::new(cold.counts.artifacts.to_string()),
            Cell::new(cold.counts.built.to_string()),
            Cell::new(cold.counts.parked.to_string()),
            Cell::colored(format!("{:.3}", millis(edited.total_micros)), "38;5;250"),
            Cell::new(edited.counts.artifacts.to_string()),
            Cell::new(edited.counts.built.to_string()),
            Cell::new(edited.counts.parked.to_string()),
            Cell::new(edited.changed_modules.unwrap_or_default().to_string()),
            Cell::colored(format!("{:.3}", millis(edited.graph_micros)), "38;5;147"),
            Cell::colored(format!("{:.3}", millis(edited.check_micros)), "38;5;170"),
            Cell::new(format!("{:.3}", millis(edited.modules_micros))),
            Cell::new(format!("{:.3}", millis(edited.edges_micros))),
            Cell::new(format!("{:.3}", millis(edited.scc_micros))),
            Cell::new(format!("{:.3}", millis(edited.condensation_micros))),
        ]);

        if graph.modules == 1000 {
            detailed_traces.push((
                format!("edit {}x{}", graph.modules, graph.component_size),
                edited_trace,
            ));
        }
    }

    table.print();

    for (name, trace) in &detailed_traces {
        TraceTable::new()
            .row(name.as_str(), trace)
            .color()
            .timeline()
            .times()
            .slow_attempts(8)
            .print();
    }
}

#[test]
fn test_open_indexes_large_source_tree() {
    let files = generated_source_tree(10_000);
    let files = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect::<Vec<_>>();
    let test = TestSession::open(&files).unwrap();

    test.assert_file_count(10_001);
}

#[test]
fn test_reload_reports_one_changed_file_in_large_tree() {
    let files = generated_source_tree(10_000);
    let files = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect::<Vec<_>>();
    let test = TestSession::open(&files).unwrap();

    test.write(
        "src/generated/file-500.ds",
        r#"export const v500 = 999;
"#,
    );
    let updates = test.reload();

    test.assert_update_paths(&updates, &["src/generated/file-500.ds"]);
    test.assert_file_count(10_001);
}

#[test]
fn test_check_rebuilds_import_chain_after_dependency_edit() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        (
            "src/index.ds",
            r#"import { value } from "./dep";

export const result = value;
"#,
        ),
        (
            "src/dep.ds",
            r#"export const value = 1;
"#,
        ),
    ])
    .unwrap();

    let (cold, cold_trace) = test.check("src/index.ds", "js");
    assert_eq!(
        TraceCounts::from_trace(&cold_trace),
        TraceCounts {
            artifacts: 5051,
            built: 3202,
            memory_cached: 0,
            store_cached: 0,
            parked: 1849,
            failed: 0,
        },
    );

    let (warm, warm_trace) = test.check("src/index.ds", "js");
    assert_eq!(warm, cold);
    assert_eq!(TraceCounts::from_trace(&warm_trace), TraceCounts::default());

    test.edit_text(
        "src/dep.ds",
        r#"export const value = 2;
"#,
    );

    let (edited, edited_trace) = test.check("src/index.ds", "js");
    assert_eq!(edited, cold);
    assert_eq!(
        TraceCounts::from_trace(&edited_trace),
        TraceCounts {
            artifacts: 18,
            built: 10,
            memory_cached: 1,
            store_cached: 0,
            parked: 7,
            failed: 0,
        },
    );
    assert_eq!(
        artifact_counter(&edited_trace, "component.graph", "changed_modules"),
        Some(1),
    );
}

#[test]
fn test_check_keeps_unrelated_module_current_after_single_edit() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        (
            "src/left.ds",
            r#"export const left = 1;
"#,
        ),
        (
            "src/right.ds",
            r#"export const right = 1;
"#,
        ),
    ])
    .unwrap();

    let (left, _) = test.check("src/left.ds", "js");
    let (right, _) = test.check("src/right.ds", "js");

    test.edit_text(
        "src/right.ds",
        r#"export const right = 2;
"#,
    );

    let (edited_left, left_trace) = test.check("src/left.ds", "js");
    let (edited_right, right_trace) = test.check("src/right.ds", "js");

    assert_eq!(edited_left, left);
    assert_ne!(edited_right, right);
    assert_eq!(
        TraceCounts::from_trace(&left_trace),
        TraceCounts {
            artifacts: 11,
            built: 6,
            memory_cached: 1,
            store_cached: 0,
            parked: 4,
            failed: 0,
        },
    );
    assert_eq!(
        TraceCounts::from_trace(&right_trace),
        TraceCounts {
            artifacts: 3,
            built: 2,
            memory_cached: 0,
            store_cached: 0,
            parked: 1,
            failed: 0,
        },
    );
}

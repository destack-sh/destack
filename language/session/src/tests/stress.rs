use tspp_repository::{TraceReport, TraceSnapshot};

use super::{
    Cell, TestSession, TextTable, TraceCounts, artifact_counter, millis, stage_micros, time_micros,
};

/// Module graph shapes used by scaling stress tests.
const SCALING_GRAPHS: [ModuleGraph; 8] = [
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

        format!("src/module-{module}.tspp")
    }

    /// Return the module edited after the cold check.
    fn edited_module(self) -> usize {
        if self.component_size > 1 {
            self.component_size / 2
        } else {
            self.modules / 2
        }
    }

    /// Return the imported module used to change one import row.
    fn import_edit_target(self) -> usize {
        let edited_module = self.edited_module();

        if edited_module == 0 { 1 } else { 0 }
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
            changed_modules: artifact_counter(trace, "component.graph", "graph.changed"),
            graph_micros: stage_micros(trace, "graph"),
            check_micros: stage_micros(trace, "check"),
            modules_micros: time_micros(trace, "modules"),
            edges_micros: time_micros(trace, "edges"),
            scc_micros: time_micros(trace, "scc"),
            condensation_micros: time_micros(trace, "condensation"),
        }
    }
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
            format!("src/module-{module}.tspp"),
            generated_module_source(graph, module, 1),
        ));
    }

    files
}

/// Build one generated source module.
fn generated_module_source(graph: ModuleGraph, module: usize, value: usize) -> String {
    generated_module_source_with_extra_import(graph, module, value, None)
}

/// Build one generated source module with one optional extra import.
fn generated_module_source_with_extra_import(
    graph: ModuleGraph,
    module: usize,
    value: usize,
    imported: Option<usize>,
) -> String {
    let mut source = String::new();
    let group_start = module / graph.component_size * graph.component_size;
    let group_end = (group_start + graph.component_size).min(graph.modules);

    // import one extra module to force a changed import row
    if let Some(imported) = imported {
        source.push_str(&format!(
            "import {{ value{imported} }} from \"./module-{imported}\";\n"
        ));
    }

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

    // export one changed value
    source.push_str(&format!("export const value{module} = {value};\n"));

    source
}

/// Build the check scaling report table.
fn check_scaling_table(title: &str) -> TextTable {
    TextTable::new().title(title).color().row(vec![
        Cell::bold("modules"),
        Cell::bold("component"),
        Cell::bold("cold ms"),
        Cell::bold("cold attempts"),
        Cell::bold("cold built"),
        Cell::bold("cold parked"),
        Cell::bold("edit ms"),
        Cell::bold("edit attempts"),
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

/// Build the component graph edit report table.
fn component_graph_edit_table(title: &str) -> TextTable {
    TextTable::new().title(title).color().row(vec![
        Cell::bold("modules"),
        Cell::bold("component"),
        Cell::bold("body ms"),
        Cell::bold("body graph ms"),
        Cell::bold("body scc ms"),
        Cell::bold("body condensation ms"),
        Cell::bold("import ms"),
        Cell::bold("import graph ms"),
        Cell::bold("import scc ms"),
        Cell::bold("import condensation ms"),
    ])
}

#[test]
fn test_measure_check_after_single_module_edit() {
    let mut table = check_scaling_table("check scaling");
    let mut detailed_traces = Vec::new();
    let worker_count = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);

    for graph in SCALING_GRAPHS {
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

        let (_cold, cold_trace) = test.check("src/module-0.tspp", "js");
        let cold = CheckMeasurement::from_trace(&cold_trace);

        let edited_source = generated_module_source(graph, graph.edited_module(), 2);
        test.edit_text(&edit_path, &edited_source);

        let (_edited, edited_trace) = test.check("src/module-0.tspp", "js");
        let edited = CheckMeasurement::from_trace(&edited_trace);

        // a body edit preserves the graph without rebuilding it
        assert_eq!(edited.changed_modules, None);
        assert_eq!(edited.counts.failed, 0);

        table = table.row(vec![
            Cell::new(graph.modules.to_string()),
            Cell::new(graph.component_size.to_string()),
            Cell::new(format!("{:.3}", millis(cold.total_micros))),
            Cell::new(cold.counts.attempts.to_string()),
            Cell::new(cold.counts.built.to_string()),
            Cell::new(cold.counts.parked.to_string()),
            Cell::colored(format!("{:.3}", millis(edited.total_micros)), "38;5;250"),
            Cell::new(edited.counts.attempts.to_string()),
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

    let mut report = TraceReport::new()
        .color()
        .timelines()
        .span_totals()
        .slow_attempts(8);
    for (name, trace) in detailed_traces {
        report = report.row(name, trace);
    }
    report.print();
}

/// Measure one component graph edit.
fn measure_component_graph_edit(graph: ModuleGraph, edited_source: String) -> CheckMeasurement {
    let files = generated_module_graph(graph);
    let files = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect::<Vec<_>>();
    let test = TestSession::open(&files).unwrap();

    let (_cold, _cold_trace) = test.check("src/module-0.tspp", "js");

    test.edit_text(&graph.edit_path(), &edited_source);

    let (_edited, edited_trace) = test.check("src/module-0.tspp", "js");

    CheckMeasurement::from_trace(&edited_trace)
}

/// Measure one body edit that leaves the import row unchanged.
fn measure_component_graph_body_edit(graph: ModuleGraph) -> CheckMeasurement {
    let edited_source = generated_module_source(graph, graph.edited_module(), 2);

    measure_component_graph_edit(graph, edited_source)
}

/// Measure one import edit that changes the import row.
fn measure_component_graph_import_edit(graph: ModuleGraph) -> CheckMeasurement {
    let module = graph.edited_module();
    let edited_source = generated_module_source_with_extra_import(
        graph,
        module,
        2,
        Some(graph.import_edit_target()),
    );

    measure_component_graph_edit(graph, edited_source)
}

#[test]
fn test_measure_component_graph_after_body_and_import_edits() {
    let mut table = component_graph_edit_table("component graph edits");

    for graph in SCALING_GRAPHS {
        println!(
            "measuring component graph edits for {} modules with component size {}",
            graph.modules, graph.component_size
        );

        let body = measure_component_graph_body_edit(graph);
        let import = measure_component_graph_import_edit(graph);

        // body edits preserve the graph through unchanged edge projections
        //  while import edits still resolve the updated graph
        assert_eq!(body.changed_modules, None);
        assert_eq!(body.counts.failed, 0);
        assert_eq!(import.counts.failed, 0);

        table = table.row(vec![
            Cell::new(graph.modules.to_string()),
            Cell::new(graph.component_size.to_string()),
            Cell::colored(format!("{:.3}", millis(body.total_micros)), "38;5;250"),
            Cell::colored(format!("{:.3}", millis(body.graph_micros)), "38;5;147"),
            Cell::new(format!("{:.3}", millis(body.scc_micros))),
            Cell::new(format!("{:.3}", millis(body.condensation_micros))),
            Cell::colored(format!("{:.3}", millis(import.total_micros)), "38;5;250"),
            Cell::colored(format!("{:.3}", millis(import.graph_micros)), "38;5;147"),
            Cell::new(format!("{:.3}", millis(import.scc_micros))),
            Cell::new(format!("{:.3}", millis(import.condensation_micros))),
        ]);
    }

    table.print();
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
            "src/index.tspp",
            r#"import { value } from "./dep";

export const result = value;
"#,
        ),
        (
            "src/dep.tspp",
            r#"export const value = 1;
"#,
        ),
    ])
    .unwrap();

    let (cold, cold_trace) = test.check("src/index.tspp", "js");
    assert_eq!(cold_trace.stats.memory_cached, 0);
    assert_eq!(cold_trace.stats.failed, 0);

    let (warm, warm_trace) = test.check("src/index.tspp", "js");
    assert_eq!(warm, cold);
    assert_eq!(TraceCounts::from_trace(&warm_trace), TraceCounts::default());

    // recheck the importer after value and documentation edits to its dependency
    let mut previous = cold;
    for content in [
        r#"export const value = 2;
"#,
        r#"export const value = 2;

/// The dependency value.
"#,
    ] {
        test.edit_text("src/dep.tspp", content);
        let (current, trace) = test.check("src/index.tspp", "js");
        assert_ne!(current, previous);
        previous = current;

        // verify exact work in the changed dependency and its importer
        let mut artifacts = trace
            .attempts
            .iter()
            .filter(|attempt| attempt.outcome != "parked")
            .map(|attempt| {
                (
                    attempt.outcome.as_str(),
                    attempt.name.as_str(),
                    attempt.label.as_deref(),
                )
            })
            .collect::<Vec<_>>();
        artifacts.sort_unstable();
        assert_eq!(
            artifacts,
            [
                ("built", "dir.bind", Some("file://src/dep.tspp")),
                ("built", "dir.check", Some("file://src/index.tspp")),
                ("built", "dir.declare", Some("file://src/dep.tspp")),
                ("built", "dir.elaborate", Some("file://src/dep.tspp")),
                ("built", "dir.expand", Some("file://src/dep.tspp")),
                ("built", "dir.export", Some("file://src/dep.tspp")),
                ("built", "dir.import", Some("file://src/dep.tspp")),
                ("built", "dir.parse", Some("file://src/dep.tspp")),
                ("built", "dir.resolve", Some("file://src/dep.tspp")),
            ],
        );
    }
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
            "src/left.tspp",
            r#"export const left = 1;
"#,
        ),
        (
            "src/right.tspp",
            r#"export const right = 1;
"#,
        ),
    ])
    .unwrap();

    let (left, _) = test.check("src/left.tspp", "js");
    let (right, _) = test.check("src/right.tspp", "js");

    test.edit_text(
        "src/right.tspp",
        r#"export const right = 2;
"#,
    );

    let (edited_left, left_trace) = test.check("src/left.tspp", "js");
    let (edited_right, right_trace) = test.check("src/right.tspp", "js");

    assert_eq!(edited_left, left);
    assert_ne!(edited_right, right);
    assert_eq!(
        TraceCounts::from_trace(&left_trace),
        TraceCounts {
            attempts: 0,
            built: 0,
            memory_cached: 0,
            parked: 0,
            failed: 0,
        },
    );
    assert_eq!(
        TraceCounts::from_trace(&right_trace),
        TraceCounts {
            attempts: 13,
            built: 9,
            memory_cached: 0,
            parked: 4,
            failed: 0,
        },
    );
}

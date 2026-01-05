use std::time::Instant;

use destack_machine::interpreter::{Interpreter, MachineOptions};
use destack_machine::memory::Value;
use destack_mir::parse::Parser;

use super::{arithmetic, calls, dispatch, intrinsics, memory};

/// Benchmark program with metadata for validation and throughput calculation.
pub(crate) struct Program {
    /// Human readable name.
    pub name: &'static str,
    /// MIR source code.
    pub source: &'static str,
    /// Entry point function name.
    pub entry: &'static str,
    /// Expected result for validation (None to skip).
    pub expected: fn() -> Option<Value>,
    /// Default arguments for benchmarking.
    pub default_args: fn() -> Vec<Value>,
}

impl Program {
    /// Create an interpreter for this program.
    pub(crate) fn interpreter(&self) -> Interpreter {
        let (tree, strings) = Parser::parse(self.source)
            .unwrap_or_else(|e| panic!("failed to parse '{}': {}", self.name, e.message));

        Interpreter::with_options(tree, strings, MachineOptions::unbounded())
    }

    /// Get actual instruction count by running once.
    #[allow(dead_code)]
    pub(crate) fn actual_instruction_count(&self, args: &[Value]) -> u64 {
        let mut interp = self.interpreter();
        let result = interp
            .run_function_by_name(self.entry, args)
            .unwrap_or_else(|e| panic!("'{}' failed: {:?}", self.name, e));
        result.statistics.instructions_executed
    }

    /// Validate that the program produces expected output.
    #[allow(dead_code)]
    pub(crate) fn validate(&self) {
        let Some(expected) = (self.expected)() else {
            return;
        };

        let mut interp = self.interpreter();
        let args = (self.default_args)();
        let result = interp
            .run_function_by_name(self.entry, &args)
            .unwrap_or_else(|e| panic!("'{}' failed: {:?}", self.name, e));

        assert_eq!(
            result.value, expected,
            "'{}': expected {:?}, got {:?}",
            self.name, expected, result.value
        );
    }
}

/// Validate all benchmark programs produce expected outputs.
#[allow(dead_code)]
pub(crate) fn validate_all() {
    for p in dispatch::ALL {
        p.validate();
    }

    for p in arithmetic::ALL {
        p.validate();
    }

    for p in calls::ALL {
        p.validate();
    }

    for p in memory::ALL {
        p.validate();
    }

    for p in intrinsics::ALL {
        p.validate();
    }
}

// ansi color codes
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RED: &str = "\x1b[31m";

/// Run quick benchmarks with ~2s total time budget.
#[allow(dead_code)]
pub(crate) fn quick_bench(filter: Option<&[&str]>) {
    use std::time::Duration;

    const TIME_PER_PROGRAM: Duration = Duration::from_millis(100);

    let all_programs: Vec<(&str, &[&Program])> = vec![
        ("dispatch", dispatch::ALL),
        ("arithmetic", arithmetic::ALL),
        ("calls", calls::ALL),
        ("memory", memory::ALL),
        ("intrinsics", intrinsics::ALL),
    ];

    // header
    println!();
    println!(
        "{BOLD}{:<28} {:>7} {:>5} {:>9} {:>6} {:>5} {:>5} {:>6} {:>5}{RESET}",
        "Program", "Mops/s", "Iter", "Instrs", "Calls", "Stk", "Alloc", "Br", "Ld/St"
    );
    println!("{DIM}{}{RESET}", "─".repeat(86));

    let bench_start = Instant::now();
    let mut total_mops = 0.0;
    let mut count = 0;

    for (category, programs) in &all_programs {
        for p in *programs {
            let full_name = format!("{}/{}", category, p.name);

            // skip if filter provided and doesn't match
            if let Some(patterns) = filter
                && !patterns.iter().any(|pat| full_name.contains(pat))
            {
                continue;
            }

            let mut interp = p.interpreter();
            let args = (p.default_args)();

            // warmup run and get stats
            let _ = interp.run_function_by_name(p.entry, &args).unwrap();
            let result = interp.run_function_by_name(p.entry, &args).unwrap();
            let stats = &result.statistics;
            let needs_gc = stats.heap_allocations > 0;

            // run for time budget
            let mut iterations = 0u32;
            let run_start = Instant::now();
            while run_start.elapsed() < TIME_PER_PROGRAM {
                if needs_gc {
                    interp.collect_garbage();
                }
                let _ = interp.run_function_by_name(p.entry, &args).unwrap();
                iterations += 1;
            }
            let elapsed = run_start.elapsed();

            let total_instructions = stats.instructions_executed * iterations as u64;
            let mops = total_instructions as f64 / elapsed.as_secs_f64() / 1_000_000.0;

            // color based on performance
            let mops_color = if mops >= 100.0 {
                GREEN
            } else if mops >= 50.0 {
                YELLOW
            } else {
                RED
            };

            let ld_st = stats.loads + stats.stores;
            println!(
                "{CYAN}{full_name:<28}{RESET} {mops_color}{mops:>7.1}{RESET} {:>5} {DIM}{:>9} {:>6} {:>5} {:>5} {:>6} {:>5}{RESET}",
                iterations,
                stats.instructions_executed,
                stats.calls_made,
                stats.max_stack_depth,
                stats.heap_allocations,
                stats.branches,
                ld_st,
            );

            total_mops += mops;
            count += 1;
        }
    }

    // summary
    let total_elapsed = bench_start.elapsed();
    println!("{DIM}{}{RESET}", "─".repeat(86));
    if count > 0 {
        println!(
            "{BOLD}{:<28} {:>7.1}{RESET}                                           {DIM}in {:.2}s{RESET}",
            "Average",
            total_mops / count as f64,
            total_elapsed.as_secs_f64()
        );
    } else {
        println!("{DIM}no programs matched filter{RESET}");
    }
    println!();
}

/// Run validation for all programs, printing results like cargo test.
#[allow(dead_code)]
pub(crate) fn quick_check() {
    let all_programs: Vec<(&str, &[&Program])> = vec![
        ("dispatch", dispatch::ALL),
        ("arithmetic", arithmetic::ALL),
        ("calls", calls::ALL),
        ("memory", memory::ALL),
        ("intrinsics", intrinsics::ALL),
    ];

    let total: usize = all_programs.iter().map(|(_, p)| p.len()).sum();
    println!();
    println!("running {total} tests");

    let start = Instant::now();
    let mut passed = 0;
    let mut skipped = 0;

    for (category, programs) in all_programs {
        for p in programs {
            print!("test {}::{} ... ", category, p.name);

            if (p.expected)().is_none() {
                println!("{YELLOW}ignored{RESET}");
                skipped += 1;
                continue;
            }

            p.validate();
            println!("{GREEN}ok{RESET}");
            passed += 1;
        }
    }

    let elapsed = start.elapsed();
    println!();
    print!("test result: {GREEN}ok{RESET}. ");
    println!(
        "{passed} passed; 0 failed; {skipped} ignored; finished in {:.2}s",
        elapsed.as_secs_f64()
    );
    println!();
}

/// Print detailed execution stats for all programs (single run).
#[allow(dead_code)]
pub(crate) fn print_stats() {
    let all_programs: Vec<(&str, &[&Program])> = vec![
        ("dispatch", dispatch::ALL),
        ("arithmetic", arithmetic::ALL),
        ("calls", calls::ALL),
        ("memory", memory::ALL),
        ("intrinsics", intrinsics::ALL),
    ];

    println!();
    println!(
        "{BOLD}{:<32} {:>12} {:>8} {:>6} {:>8}{RESET}",
        "Program", "Instructions", "Calls", "Stack", "Heap"
    );
    println!("{DIM}{}{RESET}", "─".repeat(70));

    for (category, programs) in all_programs {
        for p in programs {
            let mut interp = p.interpreter();
            let args = (p.default_args)();

            let result = interp
                .run_function_by_name(p.entry, &args)
                .expect("execution failed");

            let stats = &result.statistics;
            println!(
                "{CYAN}{:<32}{RESET} {:>12} {:>8} {:>6} {:>8}",
                format!("{}/{}", category, p.name),
                stats.instructions_executed,
                stats.calls_made,
                stats.max_stack_depth,
                stats.heap_allocations,
            );
        }
    }
    println!();
}

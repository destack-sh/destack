use std::cmp::Ordering;
use std::time::{Duration, Instant};

use destack_mir::parse::Parser;
use destack_vm::interpreter::{Interpreter, MachineOptions};
use destack_vm::memory::Value;

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

/// Benchmark options for quick runs.
pub(crate) struct BenchOptions {
    /// Filter patterns for program names.
    pub filter: Option<Vec<String>>,
    /// Number of timing repeats.
    pub repeat: u32,
    /// Minimum time per program run.
    pub min_duration: Duration,
    /// Warmup time per program.
    pub warmup: Duration,
}

#[allow(dead_code)]
impl BenchOptions {
    /// Build benchmark options from explicit values.
    pub(crate) fn new(
        filter: Option<Vec<String>>,
        repeat: u32,
        min_duration: Duration,
        warmup: Duration,
    ) -> Self {
        // assemble options
        Self {
            filter,
            repeat,
            min_duration,
            warmup,
        }
    }

    /// Build default quick benchmark options.
    pub(crate) fn quick(filter: Option<&[&str]>) -> Self {
        // normalize filter patterns
        let filter = filter.map(|patterns| patterns.iter().map(|pat| (*pat).to_string()).collect());

        // use quick defaults
        Self {
            filter,
            repeat: 3,
            min_duration: Duration::from_millis(100),
            warmup: Duration::from_millis(50),
        }
    }
}

/// Single benchmark timing sample.
struct BenchSample {
    /// Measured throughput in mops per second.
    mops: f64,
    /// Iterations performed (retained for debugging).
    #[allow(dead_code)]
    iterations: u32,
}

/// Summary of benchmark samples.
struct BenchSummary {
    /// Median mops for the sample set.
    median_mops: f64,
    /// Minimum mops observed.
    min_mops: f64,
    /// Maximum mops observed.
    max_mops: f64,
}

/// Summarize benchmark samples with median and range.
fn summarize_samples(samples: &mut [BenchSample]) -> BenchSummary {
    // handle empty samples defensively
    if samples.is_empty() {
        return BenchSummary {
            median_mops: 0.0,
            min_mops: 0.0,
            max_mops: 0.0,
        };
    }

    // sort by mops for median selection
    samples.sort_by(|a, b| a.mops.partial_cmp(&b.mops).unwrap_or(Ordering::Equal));

    // compute min and max
    let min_mops = samples.first().map(|s| s.mops).unwrap_or(0.0);
    let max_mops = samples.last().map(|s| s.mops).unwrap_or(0.0);

    // compute median mops
    let median_index = samples.len() / 2;
    let median_mops = if samples.len() % 2 == 1 {
        samples[median_index].mops
    } else {
        let lower = samples[median_index - 1].mops;
        let upper = samples[median_index].mops;
        (lower + upper) * 0.5
    };

    // return summary
    BenchSummary {
        median_mops,
        min_mops,
        max_mops,
    }
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
        result.statistics.threaded_instructions_executed
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

/// Run quick benchmarks with default settings.
#[allow(dead_code)]
pub(crate) fn quick_bench(filter: Option<&[&str]>) {
    // build quick defaults
    let options = BenchOptions::quick(filter);

    // run with defaults
    quick_bench_with_options(&options);
}

/// Run quick benchmarks with configurable timing.
#[allow(dead_code)]
pub(crate) fn quick_bench_with_options(options: &BenchOptions) {
    // gather benchmark programs
    let all_programs: Vec<(&str, &[&Program])> = vec![
        ("dispatch", dispatch::ALL),
        ("arithmetic", arithmetic::ALL),
        ("calls", calls::ALL),
        ("memory", memory::ALL),
        ("intrinsics", intrinsics::ALL),
    ];

    // clamp repeat and duration for safety
    let repeat = options.repeat.max(1);
    let min_duration = if options.min_duration.is_zero() {
        Duration::from_millis(1)
    } else {
        options.min_duration
    };

    // header
    let table_width = 100;
    println!();
    println!(
        "{BOLD}{:<28}  {:>7}  {:>8} {:>8}  {:>6} {:>4} {:>5}  {:>6} {:>5}  {:>11}{RESET}",
        "Program", "Mops/s", "MIR", "Threaded", "Calls", "Stk", "Alloc", "Br", "Mem", "Range"
    );
    if repeat > 1 {
        println!(
            "{DIM}median of {repeat} samples · min {:.0}ms · warmup {:.0}ms{RESET}",
            min_duration.as_secs_f64() * 1000.0,
            options.warmup.as_secs_f64() * 1000.0
        );
    }
    println!("{DIM}{}{RESET}", "─".repeat(table_width));

    // track summary stats
    let bench_start = Instant::now();
    let mut total_mops = 0.0;
    let mut count = 0;

    for (category, programs) in &all_programs {
        for p in *programs {
            // build display name
            let full_name = format!("{}/{}", category, p.name);

            // skip if filter provided and doesn't match
            if let Some(patterns) = options.filter.as_ref()
                && !patterns.iter().any(|pat| full_name.contains(pat))
            {
                continue;
            }

            // build interpreter and arguments
            let mut interp = p.interpreter();
            let args = (p.default_args)();

            // run once for stats
            let result = interp.run_function_by_name(p.entry, &args).unwrap();
            let stats = &result.statistics;
            let needs_gc = stats.heap_allocations > 0;

            // run warmup loop
            if options.warmup > Duration::ZERO {
                let warmup_start = Instant::now();
                while warmup_start.elapsed() < options.warmup {
                    if needs_gc {
                        interp.collect_garbage();
                    }
                    let _ = interp.run_function_by_name(p.entry, &args).unwrap();
                }
            }

            // collect timing samples
            let mut samples = Vec::with_capacity(repeat as usize);
            for _ in 0..repeat {
                // run for time budget
                let mut iterations = 0u32;
                let run_start = Instant::now();
                while run_start.elapsed() < min_duration {
                    if needs_gc {
                        interp.collect_garbage();
                    }
                    let _ = interp.run_function_by_name(p.entry, &args).unwrap();
                    iterations += 1;
                }
                let elapsed = run_start.elapsed();

                // compute throughput (based on MIR instructions for fair comparison)
                let total_instructions = stats.mir_instructions_executed * iterations as u64;
                let mops = total_instructions as f64 / elapsed.as_secs_f64() / 1_000_000.0;
                samples.push(BenchSample { mops, iterations });
            }

            // summarize samples
            let summary = summarize_samples(&mut samples);
            let range_label = if repeat > 1 {
                format!("{:.1}-{:.1}", summary.min_mops, summary.max_mops)
            } else {
                "-".to_string()
            };

            // color based on performance
            let mops_color = if summary.median_mops >= 100.0 {
                GREEN
            } else if summary.median_mops >= 50.0 {
                YELLOW
            } else {
                RED
            };

            // print benchmark row
            let mem = stats.loads + stats.stores;
            println!(
                "{CYAN}{full_name:<28}{RESET}  {mops_color}{:>7.1}{RESET}  {BOLD}{:>8}{RESET} {DIM}{:>8}  {:>6} {:>4} {:>5}  {:>6} {:>5}  {:>11}{RESET}",
                summary.median_mops,
                stats.mir_instructions_executed,
                stats.threaded_instructions_executed,
                stats.calls_made,
                stats.max_stack_depth,
                stats.heap_allocations,
                stats.branches,
                mem,
                range_label,
            );

            // update summary totals
            total_mops += summary.median_mops;
            count += 1;
        }
    }

    // summary
    let total_elapsed = bench_start.elapsed();
    println!("{DIM}{}{RESET}", "─".repeat(table_width));
    if count > 0 {
        println!(
            "{BOLD}{:<28}  {:>7.1}{RESET}  {DIM}{:>55} in {:.2}s{RESET}",
            "Average",
            total_mops / count as f64,
            "",
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
        "{BOLD}{:<32}  {:>10} {:>10}  {:>8} {:>6} {:>8}{RESET}",
        "Program", "MIR", "Threaded", "Calls", "Stack", "Heap"
    );
    println!("{DIM}{}{RESET}", "─".repeat(82));

    for (category, programs) in all_programs {
        for p in programs {
            let mut interp = p.interpreter();
            let args = (p.default_args)();

            let result = interp
                .run_function_by_name(p.entry, &args)
                .expect("execution failed");

            let stats = &result.statistics;
            println!(
                "{CYAN}{:<32}{RESET}  {:>10} {DIM}{:>10}{RESET}  {DIM}{:>8} {:>6} {:>8}{RESET}",
                format!("{}/{}", category, p.name),
                stats.mir_instructions_executed,
                stats.threaded_instructions_executed,
                stats.calls_made,
                stats.max_stack_depth,
                stats.heap_allocations,
            );
        }
    }
    println!();
}

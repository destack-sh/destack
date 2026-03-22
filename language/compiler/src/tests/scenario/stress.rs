use std::thread;

use super::{CompilerScenarioRun, CompilerScenarioWorkspace};

const REPEATS_ENV_NAME: &str = "DESTACK_SCENARIO_REPEATS";
const PARALLELISM_ENV_NAME: &str = "DESTACK_SCENARIO_PARALLELISM";
const WORKERS_ENV_NAME: &str = "DESTACK_SCENARIO_WORKERS";

/// Repeat and parallelism settings for scenario pressure runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScenarioStress {
    /// The number of sequential repetitions.
    pub repeats: usize,
    /// The number of parallel workers.
    pub parallelism: usize,
    /// The number of compiler workers per scenario run.
    pub workers: u16,
}

impl Default for ScenarioStress {
    fn default() -> Self {
        Self {
            repeats: 1,
            parallelism: 1,
            workers: 1,
        }
    }
}

impl ScenarioStress {
    /// Create default stress settings.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Set the number of sequential repetitions.
    pub(crate) fn repeats(mut self, repeats: usize) -> Self {
        self.repeats = repeats.max(1);
        self
    }

    /// Set the number of parallel workers.
    pub(crate) fn parallelism(mut self, parallelism: usize) -> Self {
        self.parallelism = parallelism.max(1);
        self
    }

    /// Set the number of compiler workers per scenario run.
    pub(crate) fn workers(mut self, workers: u16) -> Self {
        self.workers = workers.max(1);
        self
    }

    /// Apply environment overrides for ad hoc stress runs.
    pub(crate) fn with_environment_overrides(mut self) -> Self {
        // override repeats first
        if let Some(repeats) = parse_stress_env(REPEATS_ENV_NAME) {
            self.repeats = repeats;
        }

        // then override parallelism
        if let Some(parallelism) = parse_stress_env(PARALLELISM_ENV_NAME) {
            self.parallelism = parallelism;
        }

        // finally override compiler workers
        if let Some(workers) = parse_stress_env(WORKERS_ENV_NAME) {
            self.workers = workers as u16;
        }

        self
    }
}

impl CompilerScenarioRun {
    /// Run one action repeatedly on the same live compiler state.
    pub(crate) fn repeat<T, F>(&self, repeats: usize, mut action: F) -> Vec<T>
    where
        F: FnMut(usize, &Self) -> T,
    {
        // run the same action sequentially
        let mut results = Vec::with_capacity(repeats);
        for repeat in 0..repeats {
            results.push(action(repeat, self));
        }

        results
    }

    /// Run one action in parallel against the same live compiler state.
    pub(crate) fn parallel<T, F>(&self, parallelism: usize, action: F) -> Vec<T>
    where
        T: Send,
        F: Fn(usize, Self) -> T + Sync,
    {
        // spawn one scoped task per worker
        thread::scope(|scope| {
            let mut tasks = Vec::with_capacity(parallelism);

            for worker in 0..parallelism {
                let run = self.clone();
                let action = &action;
                tasks.push(scope.spawn(move || action(worker, run)));
            }

            // join the worker results in submission order
            tasks
                .into_iter()
                .map(|task| {
                    task.join()
                        .unwrap_or_else(|_| panic!("scenario worker panicked"))
                })
                .collect()
        })
    }
}

/// Parse one positive stress override from the environment.
fn parse_stress_env(name: &str) -> Option<usize> {
    // ignore missing overrides
    let value = std::env::var(name).ok()?;

    // ignore invalid or zero values
    value.parse::<usize>().ok().filter(|value| *value > 0)
}

impl CompilerScenarioWorkspace {
    /// Run one action in parallel across fresh compiler sessions.
    pub(crate) fn parallel_fresh<T, F>(&self, parallelism: usize, action: F) -> Vec<T>
    where
        T: Send,
        F: Fn(usize, CompilerScenarioRun) -> T + Sync,
    {
        // spawn one fresh scenario run per worker
        thread::scope(|scope| {
            let mut tasks = Vec::with_capacity(parallelism);

            for worker in 0..parallelism {
                let action = &action;
                tasks.push(scope.spawn(move || {
                    let run = self.open();
                    action(worker, run)
                }));
            }

            // join the worker results in submission order
            tasks
                .into_iter()
                .map(|task| {
                    task.join()
                        .unwrap_or_else(|_| panic!("scenario worker panicked"))
                })
                .collect()
        })
    }
}

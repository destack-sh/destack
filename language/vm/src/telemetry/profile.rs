#[cfg(feature = "stats")]
use std::fmt::Write;
#[cfg(feature = "stats")]
use std::time::{Duration, Instant};

/// Instruction timing profile collected via sampling.
#[cfg(feature = "stats")]
#[derive(Debug, Clone)]
pub(crate) struct InstructionProfile {
    /// Sampling interval for opcode profiling.
    sample_interval: Duration,
    /// Next sampling deadline.
    next_sample: Instant,
    /// Per-opcode sample counts.
    samples: Vec<(&'static str, u64)>,
    /// Total samples recorded.
    total_samples: u64,
}

/// Summary entry for an opcode sample.
#[cfg(feature = "stats")]
#[derive(Debug, Clone)]
pub(crate) struct InstructionProfileEntry {
    /// Opcode label.
    pub name: &'static str,
    /// Samples recorded for this opcode.
    pub samples: u64,
    /// Sample share in percent.
    pub percent: f64,
}

/// Summary for an instruction profile.
#[cfg(feature = "stats")]
#[derive(Debug, Clone)]
pub(crate) struct InstructionProfileSummary {
    /// Total samples recorded.
    pub total_samples: u64,
    /// Top sampled opcode entries.
    pub entries: Vec<InstructionProfileEntry>,
    /// Percent share for omitted entries.
    pub other_percent: f64,
}

#[cfg(feature = "stats")]
impl InstructionProfile {
    /// Create a new instruction profile sampler.
    pub(crate) fn new(sample_interval: Duration) -> Self {
        // ensure non-zero interval
        let sample_interval = if sample_interval.is_zero() {
            Duration::from_micros(100)
        } else {
            sample_interval
        };

        // initialize with empty samples
        Self {
            sample_interval,
            next_sample: Instant::now() + sample_interval,
            samples: Vec::new(),
            total_samples: 0,
        }
    }

    /// Reset collected samples while keeping the same interval.
    pub(crate) fn reset(&mut self) {
        self.next_sample = Instant::now() + self.sample_interval;
        self.samples.clear();
        self.total_samples = 0;
    }

    /// Record a sample for the given opcode if the interval elapsed.
    pub(crate) fn maybe_sample(&mut self, opcode_name: &'static str) {
        let now = Instant::now();
        if now < self.next_sample {
            return;
        }

        self.next_sample = now + self.sample_interval;
        self.total_samples += 1;

        if let Some((_, count)) = self
            .samples
            .iter_mut()
            .find(|(name, _)| *name == opcode_name)
        {
            *count += 1;
            return;
        }

        self.samples.push((opcode_name, 1));
    }

    /// Summarize sampled opcode data up to a target percent coverage.
    pub(crate) fn summary_target(&self, target_percent: f64) -> InstructionProfileSummary {
        // handle empty samples
        if self.total_samples == 0 {
            return InstructionProfileSummary {
                total_samples: 0,
                entries: Vec::new(),
                other_percent: 0.0,
            };
        }

        // build entries from samples
        let mut entries: Vec<InstructionProfileEntry> = self
            .samples
            .iter()
            .map(|(name, samples)| InstructionProfileEntry {
                name,
                samples: *samples,
                percent: *samples as f64 / self.total_samples as f64 * 100.0,
            })
            .collect();

        // sort by descending samples
        entries.sort_by(|a, b| b.samples.cmp(&a.samples));

        // select entries up to target coverage
        let target_percent = target_percent.clamp(0.0, 100.0);
        let mut selected = Vec::new();
        let mut selected_samples = 0u64;
        for entry in entries {
            selected_samples += entry.samples;
            selected.push(entry);
            let covered = selected_samples as f64 / self.total_samples as f64 * 100.0;
            if covered >= target_percent {
                break;
            }
        }

        let other_samples = self.total_samples.saturating_sub(selected_samples);
        let other_percent = if other_samples == 0 {
            0.0
        } else {
            other_samples as f64 / self.total_samples as f64 * 100.0
        };

        InstructionProfileSummary {
            total_samples: self.total_samples,
            entries: selected,
            other_percent,
        }
    }
}

#[cfg(feature = "stats")]
impl InstructionProfileSummary {
    /// Format the summary as a compact single line.
    pub(crate) fn format_compact(&self) -> String {
        if self.total_samples == 0 {
            return "no samples".to_string();
        }

        let mut out = String::new();
        for (index, entry) in self.entries.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            let _ = write!(out, "{} {:.1}%", entry.name, entry.percent);
        }
        if self.other_percent > 0.0 {
            if !out.is_empty() {
                out.push_str(", ");
            }
            let _ = write!(out, "other {:.1}%", self.other_percent);
        }

        out
    }
}

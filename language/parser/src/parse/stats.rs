#[cfg(feature = "timings")]
/// Counters for speculative parser dispatch and rollback behavior.
#[derive(Debug, Copy, Clone, Default)]
pub struct ParserSpeculationStats {
    /// The number of `with_options` scope switches.
    pub with_options_calls: u64,
    /// The number of parser rewinds.
    pub rewind_calls: u64,
    /// The number of parser restores with tree rollback.
    pub restore_calls: u64,
    /// The number of parenthesized follow-token lookups.
    pub parenthesized_follow_token_calls: u64,
    /// The number of parenthesized follow-token hits.
    pub parenthesized_follow_token_hits: u64,
    /// The number of parenthesized delimiter analysis lookups.
    pub delimiter_analysis_lookups: u64,
    /// The number of delimiter analyses that needed token-cache snapshots.
    pub delimiter_analysis_snapshot_lookups: u64,
    /// The number of delimiter analyses that executed inner scans.
    pub delimiter_analysis_scans: u64,
    /// The number of statement keyword dispatch calls.
    pub statement_keyword_dispatch_calls: u64,
    /// The number of statement keyword dispatch prefilter rejections.
    pub statement_keyword_dispatch_prefilter_rejects: u64,
    /// The number of statement keyword dispatch keyword rejections.
    pub statement_keyword_dispatch_keyword_rejects: u64,
    /// The number of direct statement keyword hits.
    pub statement_keyword_dispatch_direct_hits: u64,
    /// The number of direct statement keyword misses.
    pub statement_keyword_dispatch_direct_misses: u64,
    /// The number of parenthesized expression plain-path calls.
    pub parenthesized_expression_plain_calls: u64,
    /// The number of parenthesized expression plain-path hits.
    pub parenthesized_expression_plain_hits: u64,
    /// The number of parenthesized expression plain-path misses.
    pub parenthesized_expression_plain_misses: u64,
    /// The number of plain parenthesized lambda calls.
    pub parenthesized_lambda_plain_calls: u64,
    /// The number of plain parenthesized lambda hits.
    pub parenthesized_lambda_plain_hits: u64,
    /// The number of plain parenthesized lambda misses.
    pub parenthesized_lambda_plain_misses: u64,
    /// The number of plain identifier lambda calls.
    pub identifier_lambda_plain_calls: u64,
    /// The number of plain identifier lambda hits.
    pub identifier_lambda_plain_hits: u64,
    /// The number of plain identifier lambda misses.
    pub identifier_lambda_plain_misses: u64,
    /// The number of async keyword speculative attempts.
    pub async_keyword_speculative_attempts: u64,
    /// The number of async keyword speculative successes.
    pub async_keyword_speculative_successes: u64,
    /// The number of async keyword speculative rollbacks.
    pub async_keyword_speculative_rollbacks: u64,
}

/// Optional parser stats that are compiled only for timing builds.
#[derive(Debug, Default)]
pub(crate) struct ParserStats {
    #[cfg(feature = "timings")]
    speculation_stats: Option<ParserSpeculationStats>,
}

impl ParserStats {
    /// Create parser stats with optional speculation counters.
    pub(crate) fn new(collect_speculation_stats: bool) -> Self {
        #[cfg(not(feature = "timings"))]
        let _ = collect_speculation_stats;

        Self {
            #[cfg(feature = "timings")]
            speculation_stats: collect_speculation_stats.then(ParserSpeculationStats::default),
        }
    }

    /// Reset any enabled stats to their initial state.
    pub(crate) fn reset(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            *speculation_stats = ParserSpeculationStats::default();
        }
    }

    /// Enable or disable speculation stats.
    #[cfg(feature = "timings")]
    pub(crate) fn set_collect_speculation_stats(&mut self, enabled: bool) {
        self.speculation_stats = enabled.then(ParserSpeculationStats::default);
    }

    /// Snapshot speculation counters when they are enabled.
    #[cfg(feature = "timings")]
    pub(crate) fn speculation_snapshot(&self) -> Option<ParserSpeculationStats> {
        self.speculation_stats
    }

    /// Record one `with_options` scope switch.
    #[inline]
    pub(crate) fn record_with_options_call(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.with_options_calls += 1;
        }
    }

    /// Record one parser rewind.
    #[inline]
    pub(crate) fn record_rewind(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.rewind_calls += 1;
        }
    }

    /// Record one parser restore.
    #[inline]
    pub(crate) fn record_restore(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.restore_calls += 1;
        }
    }

    /// Record one parenthesized follow-token lookup.
    #[inline]
    pub(crate) fn record_parenthesized_follow_token_call(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_follow_token_calls += 1;
        }
    }

    /// Record one parenthesized follow-token hit.
    #[inline]
    pub(crate) fn record_parenthesized_follow_token_hit(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_follow_token_hits += 1;
        }
    }

    /// Record one delimiter analysis lookup.
    #[inline]
    pub(crate) fn record_delimiter_analysis_lookup(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.delimiter_analysis_lookups += 1;
        }
    }

    /// Record one delimiter analysis snapshot lookup.
    #[inline]
    pub(crate) fn record_delimiter_analysis_snapshot_lookup(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.delimiter_analysis_snapshot_lookups += 1;
        }
    }

    /// Record one delimiter analysis scan.
    #[inline]
    pub(crate) fn record_delimiter_analysis_scan(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.delimiter_analysis_scans += 1;
        }
    }

    /// Record one statement keyword dispatch call.
    #[inline]
    pub(crate) fn record_statement_keyword_dispatch_call(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.statement_keyword_dispatch_calls += 1;
        }
    }

    /// Record one statement keyword dispatch prefilter rejection.
    #[inline]
    pub(crate) fn record_statement_keyword_dispatch_prefilter_reject(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.statement_keyword_dispatch_prefilter_rejects += 1;
        }
    }

    /// Record one statement keyword dispatch keyword rejection.
    #[inline]
    pub(crate) fn record_statement_keyword_dispatch_keyword_reject(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.statement_keyword_dispatch_keyword_rejects += 1;
        }
    }

    /// Record one direct statement keyword hit.
    #[inline]
    pub(crate) fn record_statement_keyword_dispatch_direct_hit(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.statement_keyword_dispatch_direct_hits += 1;
        }
    }

    /// Record one direct statement keyword miss.
    #[inline]
    pub(crate) fn record_statement_keyword_dispatch_direct_miss(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.statement_keyword_dispatch_direct_misses += 1;
        }
    }

    /// Record one parenthesized expression plain-path call.
    #[inline]
    pub(crate) fn record_parenthesized_expression_plain_call(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_expression_plain_calls += 1;
        }
    }

    /// Record one parenthesized expression plain-path hit.
    #[inline]
    pub(crate) fn record_parenthesized_expression_plain_hit(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_expression_plain_hits += 1;
        }
    }

    /// Record one parenthesized expression plain-path miss.
    #[inline]
    pub(crate) fn record_parenthesized_expression_plain_miss(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_expression_plain_misses += 1;
        }
    }

    /// Record one parenthesized lambda plain-path call.
    #[inline]
    pub(crate) fn record_parenthesized_lambda_plain_call(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_lambda_plain_calls += 1;
        }
    }

    /// Record one parenthesized lambda plain-path hit.
    #[inline]
    pub(crate) fn record_parenthesized_lambda_plain_hit(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_lambda_plain_hits += 1;
        }
    }

    /// Record one parenthesized lambda plain-path miss.
    #[inline]
    pub(crate) fn record_parenthesized_lambda_plain_miss(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_lambda_plain_misses += 1;
        }
    }

    /// Record one identifier lambda plain-path call.
    #[inline]
    pub(crate) fn record_identifier_lambda_plain_call(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.identifier_lambda_plain_calls += 1;
        }
    }

    /// Record one identifier lambda plain-path hit.
    #[inline]
    pub(crate) fn record_identifier_lambda_plain_hit(&mut self) {
        #[cfg(feature = "timings")]
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.identifier_lambda_plain_hits += 1;
        }
    }
}

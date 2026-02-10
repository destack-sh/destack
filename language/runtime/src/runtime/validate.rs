use crate::diagnostic::{RuntimeError, RuntimeResult};
use destack_workspace::{RuntimeFault, RuntimeOptions, RuntimeRule};

use super::runtime::Runtime;

impl Runtime {
    /// Validate runtime options before runtime initialization.
    pub(crate) fn validate_runtime_options(options: &RuntimeOptions) -> RuntimeResult<()> {
        // validate each rule independently
        for (rule_index, rule) in options.rules.iter().enumerate() {
            validate_runtime_rule(rule, rule_index)?;
        }

        Ok(())
    }
}

/// Validate one runtime rule for structural correctness.
fn validate_runtime_rule(rule: &RuntimeRule, rule_index: usize) -> RuntimeResult<()> {
    // reject rules without any actions
    if rule.action_count() == 0 {
        return Err(RuntimeError::Internal {
            message: format!("runtime rule {rule_index} has no action"),
        }
        .boxed());
    }

    // reject rules with conflicting actions
    if rule.action_count() > 1 {
        return Err(RuntimeError::Internal {
            message: format!("runtime rule {rule_index} has more than one action"),
        }
        .boxed());
    }

    // reject empty execution selectors
    if let Some(execution_modes) = &rule.when.execution_modes
        && execution_modes.is_empty()
    {
        return Err(RuntimeError::Internal {
            message: format!("runtime rule {rule_index} has empty execution selector"),
        }
        .boxed());
    }

    // reject empty platform selectors
    if let Some(platforms) = &rule.when.platforms
        && platforms.is_empty()
    {
        return Err(RuntimeError::Internal {
            message: format!("runtime rule {rule_index} has empty platform selector"),
        }
        .boxed());
    }

    // validate fault payloads
    if let Some(fault) = &rule.fault {
        validate_runtime_fault(fault, rule_index)?;
    }

    Ok(())
}

/// Validate one runtime fault payload.
fn validate_runtime_fault(fault: &RuntimeFault, rule_index: usize) -> RuntimeResult<()> {
    // reject invalid probability bounds
    if let Some(probability_ppm) = fault.probability_ppm()
        && probability_ppm > 1_000_000
    {
        return Err(RuntimeError::Internal {
            message: format!("runtime rule {rule_index} uses probability_ppm above 1_000_000"),
        }
        .boxed());
    }

    // validate fault-specific fields
    match fault {
        // error faults require non-empty error codes
        RuntimeFault::Error { code, .. } => {
            if code.is_empty() {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "runtime rule {rule_index} error fault requires a non-empty code"
                    ),
                }
                .boxed());
            }
        }
        // delay faults must provide usable timing
        RuntimeFault::Delay {
            base_ns,
            jitter_ns,
            distribution,
            ..
        } => {
            let has_delay = *base_ns > 0;
            let has_jitter = jitter_ns.unwrap_or(0) > 0;
            if !has_delay && !has_jitter {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "runtime rule {rule_index} delay fault requires base_ns or jitter_ns"
                    ),
                }
                .boxed());
            }

            if distribution.is_some() && jitter_ns.is_none() {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "runtime rule {rule_index} delay fault requires jitter_ns when distribution is set"
                    ),
                }
                .boxed());
            }
        }
        // duplicate faults need at least one copy
        RuntimeFault::Duplicate { copies, .. } => {
            if *copies == 0 {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "runtime rule {rule_index} duplicate fault requires copies above zero"
                    ),
                }
                .boxed());
            }
        }
        // reorder faults need a usable window
        RuntimeFault::Reorder { window, .. } => {
            if *window < 2 {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "runtime rule {rule_index} reorder fault requires window of at least 2"
                    ),
                }
                .boxed());
            }
        }
        // timeout faults require positive timeout
        RuntimeFault::Timeout {
            timeout_ns, code, ..
        } => {
            if *timeout_ns == 0 {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "runtime rule {rule_index} timeout fault requires timeout_ns above zero"
                    ),
                }
                .boxed());
            }

            if let Some(code) = code
                && code.is_empty()
            {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "runtime rule {rule_index} timeout fault code cannot be empty"
                    ),
                }
                .boxed());
            }
        }
        // drop and disconnect have no required payload
        RuntimeFault::Drop { .. } | RuntimeFault::Disconnect { .. } => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Runtime;
    use destack_workspace::{
        RuntimeFault, RuntimeFaultDistribution, RuntimeOptions, RuntimeRule, RuntimeRuleFilter,
    };

    fn runtime_options_with_fault(fault: RuntimeFault) -> RuntimeOptions {
        RuntimeOptions {
            rules: vec![RuntimeRule {
                id: Some("fault-rule".to_string()),
                when: RuntimeRuleFilter::default(),
                world: None,
                access: None,
                fault: Some(fault),
            }],
            ..RuntimeOptions::default()
        }
    }

    #[test]
    fn test_reject_fault_probability_above_one_million() {
        let options = runtime_options_with_fault(RuntimeFault::Drop {
            probability_ppm: Some(1_000_001),
        });
        let result = Runtime::validate_runtime_options(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_delay_fault_without_delay_or_jitter() {
        let options = runtime_options_with_fault(RuntimeFault::Delay {
            base_ns: 0,
            jitter_ns: None,
            distribution: None,
            probability_ppm: None,
        });
        let result = Runtime::validate_runtime_options(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_delay_distribution_without_jitter() {
        let options = runtime_options_with_fault(RuntimeFault::Delay {
            base_ns: 100,
            jitter_ns: None,
            distribution: Some(RuntimeFaultDistribution::Uniform),
            probability_ppm: None,
        });
        let result = Runtime::validate_runtime_options(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_accept_delay_fault_with_jitter() {
        let options = runtime_options_with_fault(RuntimeFault::Delay {
            base_ns: 1_000,
            jitter_ns: Some(250),
            distribution: Some(RuntimeFaultDistribution::Normal),
            probability_ppm: Some(50_000),
        });
        let result = Runtime::validate_runtime_options(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_duplicate_fault_with_zero_copies() {
        let options = runtime_options_with_fault(RuntimeFault::Duplicate {
            copies: 0,
            probability_ppm: None,
        });
        let result = Runtime::validate_runtime_options(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_reorder_fault_with_small_window() {
        let options = runtime_options_with_fault(RuntimeFault::Reorder {
            window: 1,
            probability_ppm: None,
        });
        let result = Runtime::validate_runtime_options(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_timeout_fault_with_zero_timeout() {
        let options = runtime_options_with_fault(RuntimeFault::Timeout {
            timeout_ns: 0,
            code: None,
            probability_ppm: None,
        });
        let result = Runtime::validate_runtime_options(&options);
        assert!(result.is_err());
    }
}

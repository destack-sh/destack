use crate::lsp::{
    NormalizedCompletionItem, NormalizedLocation, NormalizedQuickInfo, NormalizedWorkspaceSymbol,
};

use super::core::{StressAnchor, StressOperationExpectation, StressProject};

/// One normalized IDE battery over the shared stress project model.
#[derive(Debug, Clone)]
pub(super) struct StressIdeBattery {
    /// The anchors covered by this battery.
    pub anchors: Vec<StressAnchor>,
    /// The workspace queries covered by this battery.
    pub workspace_queries: Vec<String>,
}

/// One normalized IDE driver over the shared stress project model.
pub(super) trait StressIdeDriver {
    /// Return the driver label for diagnostics.
    fn label(&self) -> &'static str;

    /// Resolve hover quick info for one generated anchor.
    fn hover(&mut self, anchor: &StressAnchor) -> Result<Option<NormalizedQuickInfo>, String>;

    /// Resolve goto definition for one generated anchor.
    fn goto_definition(&mut self, anchor: &StressAnchor)
    -> Result<Vec<NormalizedLocation>, String>;

    /// Resolve goto declaration for one generated anchor.
    fn goto_declaration(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedLocation>, String>;

    /// Resolve goto type definition for one generated anchor.
    fn goto_type_definition(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedLocation>, String>;

    /// Resolve references for one generated anchor.
    fn references(&mut self, anchor: &StressAnchor) -> Result<Vec<NormalizedLocation>, String>;

    /// Resolve completion items for one generated anchor.
    fn completion(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedCompletionItem>, String>;

    /// Resolve workspace symbols for one generated query string.
    fn workspace_symbols(&mut self, query: &str) -> Result<Vec<NormalizedWorkspaceSymbol>, String>;
}

impl StressIdeBattery {
    /// Build one full battery from one generated project.
    pub(super) fn full(project: &StressProject) -> Self {
        Self {
            anchors: project.anchors.clone(),
            workspace_queries: project.workspace_queries.clone(),
        }
    }
}

/// Run the shared deterministic IDE battery over one generated project.
pub(super) fn run_ide_invariants(
    driver: &mut impl StressIdeDriver,
    battery: &StressIdeBattery,
) -> Result<(), String> {
    for anchor in &battery.anchors {
        run_anchor_invariants(driver, anchor)?;
    }

    for workspace_query in &battery.workspace_queries {
        let first = driver.workspace_symbols(workspace_query)?;
        let second = driver.workspace_symbols(workspace_query)?;

        // workspace symbols
        assert_repeated_result_is_stable(
            driver.label(),
            "workspace symbols",
            workspace_query,
            &first,
            &second,
        )?;
        assert_result_is_non_empty(driver.label(), "workspace symbols", workspace_query, &first)?;
    }

    Ok(())
}

/// Run exact parity between two IDE drivers over one generated project.
pub(super) fn run_ide_parity(
    left: &mut impl StressIdeDriver,
    right: &mut impl StressIdeDriver,
    battery: &StressIdeBattery,
) -> Result<(), String> {
    for anchor in &battery.anchors {
        run_anchor_parity(left, right, anchor)?;
    }

    for workspace_query in &battery.workspace_queries {
        let left_symbols = left.workspace_symbols(workspace_query)?;
        let right_symbols = right.workspace_symbols(workspace_query)?;

        // workspace symbol parity
        if left_symbols != right_symbols {
            return Err(format!(
                "workspace symbols parity mismatch for query '{workspace_query}'\nleft({}): {left_symbols:#?}\nright({}): {right_symbols:#?}",
                left.label(),
                right.label(),
            ));
        }
    }

    Ok(())
}

/// Run deterministic checks for one generated anchor.
fn run_anchor_invariants(
    driver: &mut impl StressIdeDriver,
    anchor: &StressAnchor,
) -> Result<(), String> {
    let context = format!("{} ({:?})", anchor.name, anchor.kind);
    let expectations = anchor.expectations;

    assert_optional_scalar_expectation(
        driver.label(),
        "hover",
        &context,
        expectations.hover,
        || driver.hover(anchor),
    )?;
    assert_collection_expectation(
        driver.label(),
        "goto definition",
        &context,
        expectations.definition,
        || driver.goto_definition(anchor),
    )?;
    assert_collection_expectation(
        driver.label(),
        "goto declaration",
        &context,
        expectations.declaration,
        || driver.goto_declaration(anchor),
    )?;
    assert_collection_expectation(
        driver.label(),
        "goto type definition",
        &context,
        expectations.type_definition,
        || driver.goto_type_definition(anchor),
    )?;
    assert_collection_expectation(
        driver.label(),
        "references",
        &context,
        expectations.references,
        || driver.references(anchor),
    )?;
    assert_collection_expectation(
        driver.label(),
        "completion",
        &context,
        expectations.completion,
        || driver.completion(anchor),
    )?;

    Ok(())
}

/// Run exact parity for one generated anchor.
fn run_anchor_parity(
    left: &mut impl StressIdeDriver,
    right: &mut impl StressIdeDriver,
    anchor: &StressAnchor,
) -> Result<(), String> {
    let context = format!("{} ({:?})", anchor.name, anchor.kind);
    let expectations = anchor.expectations;

    assert_parity_when_checked(
        left,
        right,
        "hover",
        &context,
        expectations.hover,
        |driver| driver.hover(anchor),
    )?;
    assert_parity_when_checked(
        left,
        right,
        "goto definition",
        &context,
        expectations.definition,
        |driver| driver.goto_definition(anchor),
    )?;
    assert_parity_when_checked(
        left,
        right,
        "goto declaration",
        &context,
        expectations.declaration,
        |driver| driver.goto_declaration(anchor),
    )?;
    assert_parity_when_checked(
        left,
        right,
        "goto type definition",
        &context,
        expectations.type_definition,
        |driver| driver.goto_type_definition(anchor),
    )?;
    assert_parity_when_checked(
        left,
        right,
        "references",
        &context,
        expectations.references,
        |driver| driver.references(anchor),
    )?;
    assert_parity_when_checked(
        left,
        right,
        "completion",
        &context,
        expectations.completion,
        |driver| driver.completion(anchor),
    )?;

    Ok(())
}

/// Assert one scalar optional result against the configured expectation.
fn assert_optional_scalar_expectation<T: std::fmt::Debug + PartialEq>(
    driver_label: &str,
    operation: &str,
    context: &str,
    expectation: StressOperationExpectation,
    mut run: impl FnMut() -> Result<Option<T>, String>,
) -> Result<(), String> {
    if expectation == StressOperationExpectation::Skip {
        return Ok(());
    }

    let first = run()?;
    let second = run()?;

    assert_repeated_result_is_stable(driver_label, operation, context, &first, &second)?;

    match expectation {
        StressOperationExpectation::Skip => Ok(()),
        StressOperationExpectation::Forbidden => {
            if first.is_none() {
                Ok(())
            } else {
                Err(format!(
                    "{driver_label} {operation} unexpectedly returned a result at {context}: {first:#?}"
                ))
            }
        }
        StressOperationExpectation::Optional => Ok(()),
        StressOperationExpectation::Required => {
            if first.is_some() {
                Ok(())
            } else {
                Err(format!(
                    "{driver_label} {operation} did not return a result at {context}"
                ))
            }
        }
    }
}

/// Assert one collection result against the configured expectation.
fn assert_collection_expectation<T: std::fmt::Debug + PartialEq>(
    driver_label: &str,
    operation: &str,
    context: &str,
    expectation: StressOperationExpectation,
    mut run: impl FnMut() -> Result<Vec<T>, String>,
) -> Result<(), String> {
    if expectation == StressOperationExpectation::Skip {
        return Ok(());
    }

    let first = run()?;
    let second = run()?;

    assert_repeated_result_is_stable(driver_label, operation, context, &first, &second)?;

    match expectation {
        StressOperationExpectation::Skip => Ok(()),
        StressOperationExpectation::Forbidden => {
            if first.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "{driver_label} {operation} unexpectedly returned results at {context}: {first:#?}"
                ))
            }
        }
        StressOperationExpectation::Optional => Ok(()),
        StressOperationExpectation::Required => {
            assert_result_is_non_empty(driver_label, operation, context, &first)
        }
    }
}

/// Compare one operation across both drivers when the expectation checks it.
fn assert_parity_when_checked<T: std::fmt::Debug + PartialEq>(
    left: &mut impl StressIdeDriver,
    right: &mut impl StressIdeDriver,
    operation: &str,
    context: &str,
    expectation: StressOperationExpectation,
    run: impl Fn(&mut dyn StressIdeDriver) -> Result<T, String>,
) -> Result<(), String> {
    if expectation == StressOperationExpectation::Skip {
        return Ok(());
    }

    assert_exact_parity(left, right, operation, context, run)
}

/// Compare one normalized operation across two IDE drivers.
fn assert_exact_parity<T: std::fmt::Debug + PartialEq>(
    left: &mut impl StressIdeDriver,
    right: &mut impl StressIdeDriver,
    operation: &str,
    context: &str,
    run: impl Fn(&mut dyn StressIdeDriver) -> Result<T, String>,
) -> Result<(), String> {
    let left_value = run(left)?;
    let right_value = run(right)?;

    // exact parity
    if left_value != right_value {
        return Err(format!(
            "{operation} parity mismatch at {context}\nleft({}): {left_value:#?}\nright({}): {right_value:#?}",
            left.label(),
            right.label(),
        ));
    }

    Ok(())
}

/// Ensure one operation stays deterministic.
fn assert_repeated_result_is_stable<T: std::fmt::Debug + PartialEq>(
    driver_label: &str,
    operation: &str,
    context: &str,
    first: &T,
    second: &T,
) -> Result<(), String> {
    if first != second {
        return Err(format!(
            "{driver_label} {operation} is not deterministic at {context}\nfirst: {first:#?}\nsecond: {second:#?}",
        ));
    }

    Ok(())
}

/// Ensure one normalized collection is not empty.
fn assert_result_is_non_empty<T>(
    driver_label: &str,
    operation: &str,
    context: &str,
    values: &[T],
) -> Result<(), String> {
    if values.is_empty() {
        return Err(format!(
            "{driver_label} {operation} returned no results at {context}"
        ));
    }

    Ok(())
}

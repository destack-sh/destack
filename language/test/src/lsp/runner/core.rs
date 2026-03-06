use std::path::Path;

use crate::harness::TestResult;
use crate::lsp::runner::{
    assists, commands, hierarchy, lifecycle, navigation, refactor, symbols, tokens,
};
use crate::lsp::{LspFixture, LspTestState, Marker, NormalizedLocation, Range};
use crate::mdtest::MdTestCase;

/// Run one applied-LSP markdown test case.
pub(crate) fn run_mdtest_case(path: &Path, test: &MdTestCase) -> TestResult {
    // parse the fixture before higher-level scenario execution begins
    let fixture = match LspFixture::from_mdtest(path, test) {
        Ok(fixture) => fixture,
        Err(error) => {
            return TestResult::Failed {
                message: error.to_string(),
            };
        }
    };

    // bootstrap the editor-shaped test state against the materialized workspace
    let mut test_state = match LspTestState::from_fixture("applied_lsp", &fixture) {
        Ok(test_state) => test_state,
        Err(error) => {
            return TestResult::Failed { message: error };
        }
    };

    // open the declared fixture files before semantic requests run
    if let Err(error) = test_state.open_fixture_files() {
        return TestResult::Failed { message: error };
    }

    // execute declared workspace commands before semantic request families run
    if let Err(error) = commands::run_command_cases(&fixture, &mut test_state) {
        return TestResult::Failed { message: error };
    }

    // run the capability families in a stable order
    if let Err(error) = navigation::run_navigation_cases(&fixture, &mut test_state) {
        return TestResult::Failed { message: error };
    }

    if let Err(error) = assists::run_assist_cases(&fixture, &mut test_state) {
        return TestResult::Failed { message: error };
    }

    if let Err(error) = hierarchy::run_hierarchy_cases(&fixture, &mut test_state) {
        return TestResult::Failed { message: error };
    }

    if let Err(error) = symbols::run_symbol_cases(&fixture, &mut test_state) {
        return TestResult::Failed { message: error };
    }

    if let Err(error) = tokens::run_token_cases(&fixture, &mut test_state) {
        return TestResult::Failed { message: error };
    }

    if let Err(error) = refactor::run_refactor_cases(&fixture, &mut test_state) {
        return TestResult::Failed { message: error };
    }

    if let Err(error) = lifecycle::run_lifecycle_cases(&fixture, &mut test_state) {
        return TestResult::Failed { message: error };
    }

    TestResult::Passed
}

/// Build one expected normalized location from a definition marker.
pub(crate) fn expected_location_from_marker(
    fixture: &LspFixture,
    marker: &Marker,
) -> Result<NormalizedLocation, String> {
    let definition_width = symbol_width_at_marker(fixture, marker)
        .ok_or_else(|| "failed to derive symbol width at definition marker".to_string())?;

    Ok(NormalizedLocation {
        file_path: marker.file_path.clone(),
        start_line: marker.line,
        start_character: marker.character,
        end_line: marker.line,
        end_character: marker.character + definition_width,
    })
}

/// Return the parsed range that contains the marker position.
pub(crate) fn range_for_marker<'a>(fixture: &'a LspFixture, marker: &Marker) -> Option<&'a Range> {
    fixture
        .ranges
        .iter()
        .filter(|range| {
            range.file_path == marker.file_path
                && range.start_offset <= marker.offset
                && marker.offset <= range.end_offset
        })
        .min_by_key(|range| range.end_offset - range.start_offset)
}

/// Return the primary file path for one fixture.
pub(crate) fn primary_file_path(fixture: &LspFixture) -> Result<String, String> {
    fixture.first_file_path()
}

/// Measure the identifier-like symbol width at one marker position.
fn symbol_width_at_marker(fixture: &LspFixture, marker: &Marker) -> Option<usize> {
    let file = fixture.file(&marker.file_path)?;
    let tail = file.text.get(marker.offset..)?;
    let width = tail
        .chars()
        .take_while(|character: &char| character.is_ascii_alphanumeric() || *character == '_')
        .count();

    (width > 0).then_some(width)
}

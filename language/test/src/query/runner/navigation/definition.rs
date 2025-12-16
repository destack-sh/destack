use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::QueryTestSession;

/// Run a goto_definition test.
///
/// For each use marker, verify it resolves to the expected def marker.
pub fn run(session: &QueryTestSession) -> TestResult {
    for reference in session.markers.references() {
        let Some(target_name) = &reference.target else {
            continue;
        };

        let Some(expected_def) = session.markers.range(target_name) else {
            return TestResult::Failed {
                message: format!("target marker '{}' not found", target_name),
            };
        };

        // get the offset at the start of the reference
        let offset = reference.span.start;

        // call goto_definition
        let result = query::goto_definition(&session.session, session.file_id, offset);

        match result {
            Some(def_result) => {
                if def_result.locations.is_empty() {
                    return TestResult::Failed {
                        message: format!(
                            "goto_definition at offset {offset} returned empty result"
                        ),
                    };
                }

                // check if any location matches expected
                let found_match = def_result.locations.iter().any(|loc| {
                    loc.start == expected_def.span.start && loc.end == expected_def.span.end
                });

                if !found_match {
                    return TestResult::Failed {
                        message: format!(
                            "goto_definition at offset {} returned wrong location: expected {:?}, got {:?}",
                            offset, expected_def.span, def_result.locations[0]
                        ),
                    };
                }
            }
            None => {
                return TestResult::Failed {
                    message: format!("goto_definition at offset {offset} returned None"),
                };
            }
        }
    }

    TestResult::Passed
}

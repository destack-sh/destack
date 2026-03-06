use crate::lsp::runner::expected_location_from_marker;
use crate::lsp::{
    LspFixture, LspScenario, LspTestState, NormalizedLocation, normalize_definition_response,
    normalize_document_highlights, normalize_document_links, normalize_references_response,
    verify_definition_locations, verify_document_links, verify_exact_eq,
    verify_reference_locations,
};

/// Run the navigation scenarios declared by one fixture.
pub(crate) fn run_navigation_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // each-marker definitions
    if fixture.has_scenario(LspScenario::DefinitionEachMarkerCrossModule) {
        let def_marker = test_state.test().marker_by_name("def")?;
        let expected_locations = vec![expected_location_from_marker(fixture, &def_marker)?];
        let marker_names = test_state.test().marker_names();
        let expected_marker_names = vec![
            "def".to_string(),
            "use_first".to_string(),
            "use_second".to_string(),
        ];

        verify_exact_eq("marker names", &marker_names, &expected_marker_names)?;

        test_state.go_to().each_named_marker(
            &["use_first", "use_second"],
            |test_state, _, _| {
                let definition = test_state
                    .request_definition()?
                    .ok_or_else(|| "expected goto definition result".to_string())?;
                let actual_locations =
                    normalize_definition_response(test_state.workspace_root(), &definition)?;

                verify_definition_locations(&actual_locations, &expected_locations)
            },
        )?;
    }

    // definition
    if test_state.marker("use").is_some() {
        let def_marker = fixture
            .marker("def")
            .ok_or_else(|| "definition fixture is missing /*def*/ marker".to_string())?;
        let expected_locations = vec![expected_location_from_marker(fixture, def_marker)?];

        test_state.go_to_marker("use")?;
        let definition = test_state
            .request_definition()?
            .ok_or_else(|| "expected goto definition result".to_string())?;
        let actual_locations =
            normalize_definition_response(test_state.workspace_root(), &definition)?;

        verify_definition_locations(&actual_locations, &expected_locations)?;
    }

    // references
    if test_state.marker("refs").is_some() {
        let expected_locations = fixture
            .ranges
            .iter()
            .map(|range| NormalizedLocation {
                file_path: range.file_path.clone(),
                start_line: range.start_line,
                start_character: range.start_character,
                end_line: range.end_line,
                end_character: range.end_character,
            })
            .collect::<Vec<_>>();

        test_state.go_to_marker("refs")?;
        let references = test_state
            .request_references(true)?
            .ok_or_else(|| "expected references result".to_string())?;
        let actual_locations =
            normalize_references_response(test_state.workspace_root(), &references)?;

        verify_reference_locations(&actual_locations, &expected_locations)?;
    }

    // each-range references
    if fixture.has_scenario(LspScenario::ReferencesEachRangeCrossModule) {
        let lib_ranges = test_state.test().ranges_in_file(Some("lib.ds"));
        let main_ranges = test_state.test().ranges_in_file(Some("main.ds"));
        let lib_spans = test_state.test().spans(Some("lib.ds"));
        let main_spans = test_state.test().spans(Some("main.ds"));
        let value_ranges = test_state
            .test()
            .ranges_by_text()
            .remove("value")
            .ok_or_else(|| "fixture is missing grouped ranges for text \"value\"".to_string())?;
        let expected_locations = value_ranges
            .into_iter()
            .map(|range| NormalizedLocation {
                file_path: range.file_path.clone(),
                start_line: range.start_line,
                start_character: range.start_character,
                end_line: range.end_line,
                end_character: range.end_character,
            })
            .collect::<Vec<_>>();

        verify_exact_eq(
            "range counts",
            &(lib_ranges.len(), main_ranges.len()),
            &(1, 3),
        )?;
        verify_exact_eq("span counts", &(lib_spans.len(), main_spans.len()), &(1, 3))?;
        test_state.verify().assert_has_ranges(&fixture.ranges)?;

        test_state.go_to().each_range(|test_state, _, _| {
            let references = test_state
                .request_references(true)?
                .ok_or_else(|| "expected references result".to_string())?;
            let actual_locations =
                normalize_references_response(test_state.workspace_root(), &references)?;

            verify_reference_locations(&actual_locations, &expected_locations)
        })?;
    }

    // each-marker definitions over every marker in source order
    if fixture.has_scenario(LspScenario::DefinitionEachMarkerSameFile) {
        let definition_range = fixture
            .range_by_text("value")
            .ok_or_else(|| "definition fixture is missing [|value|] range".to_string())?;
        let expected_locations = vec![NormalizedLocation {
            file_path: definition_range.file_path.clone(),
            start_line: definition_range.start_line,
            start_character: definition_range.start_character,
            end_line: definition_range.end_line,
            end_character: definition_range.end_character,
        }];

        test_state.go_to().each_marker(|test_state, _, _| {
            let definition = test_state
                .request_definition()?
                .ok_or_else(|| "expected goto definition result".to_string())?;
            let actual_locations =
                normalize_definition_response(test_state.workspace_root(), &definition)?;

            verify_definition_locations(&actual_locations, &expected_locations)
        })?;
    }

    // declaration
    if test_state.marker("declare_use").is_some() {
        let declaration_marker = fixture
            .marker("declare_def")
            .ok_or_else(|| "declaration fixture is missing /*declare_def*/ marker".to_string())?;
        let expected_locations = vec![expected_location_from_marker(fixture, declaration_marker)?];

        test_state.go_to_marker("declare_use")?;
        let declaration = test_state
            .request_declaration()?
            .ok_or_else(|| "expected goto declaration result".to_string())?;
        let actual_locations =
            normalize_definition_response(test_state.workspace_root(), &declaration)?;

        verify_definition_locations(&actual_locations, &expected_locations)?;
    }

    // type definition
    if test_state.marker("type_use").is_some() {
        let type_marker = fixture
            .marker("type_def")
            .ok_or_else(|| "type definition fixture is missing /*type_def*/ marker".to_string())?;
        let expected_locations = vec![expected_location_from_marker(fixture, type_marker)?];

        test_state.go_to_marker("type_use")?;
        let type_definition = test_state
            .request_type_definition()?
            .ok_or_else(|| "expected goto type definition result".to_string())?;
        let actual_locations =
            normalize_definition_response(test_state.workspace_root(), &type_definition)?;

        verify_definition_locations(&actual_locations, &expected_locations)?;
    }

    // implementation
    if test_state.marker("impl_use").is_some() {
        let expected_locations = fixture
            .ranges
            .iter()
            .map(|range| NormalizedLocation {
                file_path: range.file_path.clone(),
                start_line: range.start_line,
                start_character: range.start_character,
                end_line: range.end_line,
                end_character: range.end_character,
            })
            .collect::<Vec<_>>();

        test_state.go_to_marker("impl_use")?;
        let implementation = test_state
            .request_implementation()?
            .ok_or_else(|| "expected goto implementation result".to_string())?;
        let actual_locations =
            normalize_definition_response(test_state.workspace_root(), &implementation)?;

        verify_definition_locations(&actual_locations, &expected_locations)?;
    }

    // document highlights
    if test_state.marker("highlight").is_some() {
        let expected_locations = fixture
            .ranges
            .iter()
            .map(|range| NormalizedLocation {
                file_path: range.file_path.clone(),
                start_line: range.start_line,
                start_character: range.start_character,
                end_line: range.end_line,
                end_character: range.end_character,
            })
            .collect::<Vec<_>>();

        test_state.go_to_marker("highlight")?;
        let highlights = test_state
            .request_document_highlights()?
            .ok_or_else(|| "expected document highlights result".to_string())?;
        let active_file_path = test_state
            .active_file_path()
            .ok_or_else(|| "document highlight request did not keep an active file".to_string())?;
        let actual_locations = normalize_document_highlights(&active_file_path, &highlights);

        verify_reference_locations(&actual_locations, &expected_locations)?;
    }

    // document links
    if let Some(expected_snapshot) = fixture.expectations.document_links.snapshot_text.as_deref() {
        let expected_links = crate::lsp::parse_expected_document_links(expected_snapshot)?;
        let active_file_path = fixture.first_file_path()?;

        test_state.go_to().file(&active_file_path)?;
        let links = test_state
            .request_document_links()?
            .ok_or_else(|| "expected document links result".to_string())?;
        let actual_links = normalize_document_links(test_state.workspace_root(), &links)?;

        verify_document_links(&actual_links, &expected_links)?;

        // resolve the first eager link to keep the no-op resolve path covered
        if let Some(first_link) = links.into_iter().next() {
            let resolved_link = test_state
                .resolve_document_link(first_link)?
                .ok_or_else(|| "expected document link resolve result".to_string())?;
            let actual_resolved =
                normalize_document_links(test_state.workspace_root(), &[resolved_link])?;
            let expected_resolved = expected_links
                .first()
                .cloned()
                .ok_or_else(|| "document link fixture is missing expected link".to_string())?;

            verify_document_links(&actual_resolved, &[expected_resolved])?;
        }
    }

    Ok(())
}

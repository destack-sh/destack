use crate::lsp::runner::{expected_location_from_marker, range_for_marker};
use crate::lsp::{
    ExpectedDocumentSymbol, ExpectedWorkspaceSymbol, LspFixture, LspTestState, Marker,
    NormalizedDocumentSymbol, NormalizedLocation, NormalizedWorkspaceSymbol,
    expected_symbol_kind_name, normalize_document_symbols, normalize_workspace_symbols,
    verify_document_symbols, verify_workspace_symbols,
};

/// Run the symbol cases declared by one fixture.
pub(crate) fn run_symbol_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // document symbols
    if !fixture.expectations.symbols.document.is_empty() {
        let active_file_path = test_state
            .fixture
            .files
            .first()
            .map(|file| file.path.clone())
            .ok_or_else(|| "document symbol fixture is missing a file".to_string())?;
        let expected_symbols = expected_document_symbols(fixture)?;

        test_state.go_to().file(&active_file_path)?;

        let document_symbols = test_state
            .request_document_symbols()?
            .ok_or_else(|| "expected document symbols result".to_string())?;
        let actual_symbols = normalize_document_symbols(
            test_state.workspace_root(),
            &active_file_path,
            &document_symbols,
        )?;

        verify_document_symbols(&actual_symbols, &expected_symbols)?;
    }

    // workspace symbols
    if let Some(query) = fixture.expectations.symbols.workspace_query.as_deref() {
        let expected_symbols = expected_workspace_symbols(fixture)?;
        let workspace_symbols = test_state.request_workspace_symbols(query)?;
        let actual_symbols = match workspace_symbols {
            Some(response) => normalize_workspace_symbols(test_state.workspace_root(), &response)?,
            None => Vec::new(),
        };

        verify_workspace_symbols(&actual_symbols, &expected_symbols)?;
    }

    Ok(())
}

/// Build exact expected document symbols from directive metadata plus source ranges.
fn expected_document_symbols(
    fixture: &LspFixture,
) -> Result<Vec<NormalizedDocumentSymbol>, String> {
    let mut entries = fixture
        .expectations
        .symbols
        .document
        .iter()
        .map(|symbol| expected_document_symbol(fixture, symbol))
        .collect::<Result<Vec<_>, _>>()?;

    // derive hierarchy depth from the smallest containing symbol range
    for index in 0..entries.len() {
        let depth = entries
            .iter()
            .enumerate()
            .filter(|(parent_index, parent)| {
                *parent_index != index && parent.range.contains(&entries[index].range)
            })
            .count();
        entries[index].depth = depth;
    }

    // compare document symbols in source order
    entries.sort_by_key(|symbol| {
        (
            symbol.range.file_path.clone(),
            symbol.range.start_line,
            symbol.range.start_character,
            symbol.depth,
        )
    });

    Ok(entries)
}

/// Build one expected normalized document symbol from its directive metadata.
fn expected_document_symbol(
    fixture: &LspFixture,
    symbol: &ExpectedDocumentSymbol,
) -> Result<NormalizedDocumentSymbol, String> {
    let marker = fixture.marker(&symbol.marker_name).ok_or_else(|| {
        format!(
            "document symbol fixture is missing /*{}*/ marker",
            symbol.marker_name
        )
    })?;
    let range = range_for_marker(fixture, marker).ok_or_else(|| {
        format!(
            "document symbol fixture is missing [|...|] range around /*{}*/",
            symbol.marker_name
        )
    })?;
    let selection_range = expected_location_from_marker(fixture, marker)?;

    Ok(NormalizedDocumentSymbol {
        depth: 0,
        name: symbol_name_for_marker(fixture, marker)?,
        kind: expected_symbol_kind_name(&symbol.kind)?,
        range: NormalizedLocation {
            file_path: range.file_path.clone(),
            start_line: range.start_line,
            start_character: range.start_character,
            end_line: range.end_line,
            end_character: range.end_character,
        },
        selection_range,
    })
}

/// Build exact expected workspace symbols from directive metadata plus source ranges.
fn expected_workspace_symbols(
    fixture: &LspFixture,
) -> Result<Vec<NormalizedWorkspaceSymbol>, String> {
    fixture
        .expectations
        .symbols
        .workspace
        .iter()
        .map(|symbol| expected_workspace_symbol(fixture, symbol))
        .collect()
}

/// Build one expected normalized workspace symbol from its directive metadata.
fn expected_workspace_symbol(
    fixture: &LspFixture,
    symbol: &ExpectedWorkspaceSymbol,
) -> Result<NormalizedWorkspaceSymbol, String> {
    let marker = fixture.marker(&symbol.marker_name).ok_or_else(|| {
        format!(
            "workspace symbol fixture is missing /*{}*/ marker",
            symbol.marker_name
        )
    })?;
    let range = range_for_marker(fixture, marker).ok_or_else(|| {
        format!(
            "workspace symbol fixture is missing [|...|] range around /*{}*/",
            symbol.marker_name
        )
    })?;

    Ok(NormalizedWorkspaceSymbol {
        name: symbol_name_for_marker(fixture, marker)?,
        kind: expected_symbol_kind_name(&symbol.kind)?,
        location: NormalizedLocation {
            file_path: range.file_path.clone(),
            start_line: range.start_line,
            start_character: range.start_character,
            end_line: range.end_line,
            end_character: range.end_character,
        },
        container_name: symbol.container_name.clone(),
    })
}

/// Read one identifier-like symbol name starting at the marker offset.
fn symbol_name_for_marker(fixture: &LspFixture, marker: &Marker) -> Result<String, String> {
    let file = fixture
        .file(&marker.file_path)
        .ok_or_else(|| format!("fixture is missing file {}", marker.file_path))?;
    let tail = file
        .text
        .get(marker.offset..)
        .ok_or_else(|| format!("marker {} does not align to a char boundary", marker.name))?;
    let width = tail
        .chars()
        .take_while(|character: &char| character.is_ascii_alphanumeric() || *character == '_')
        .count();
    if width == 0 {
        return Err(format!(
            "failed to derive symbol width at marker {}",
            marker.name
        ));
    }

    Ok(tail.chars().take(width).collect())
}

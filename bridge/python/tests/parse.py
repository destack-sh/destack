from session import open_workspace


def test_parse_returns_clean_module() -> None:
    fixture = open_workspace(
        [
            ("destack.json", '{"name":"@test/app"}'),
            ("src/index.ds", "export const value = 1;"),
        ],
    )
    workspace = fixture.workspace
    revision = workspace.revision()
    module = workspace.module("src/index.ds")

    # parse one loaded source module
    output = workspace.parse(revision, module)

    assert output.diagnostics == []


def test_parse_returns_syntax_error() -> None:
    fixture = open_workspace(
        [
            ("destack.json", '{"name":"@test/app"}'),
            ("src/index.ds", "export const broken ="),
        ],
    )
    workspace = fixture.workspace
    revision = workspace.revision()
    module = workspace.module("src/index.ds")

    # parse and inspect the returned syntax diagnostic
    output = workspace.parse(revision, module)

    assert len(output.diagnostics) == 1
    diagnostic = output.diagnostics[0]
    assert diagnostic.code == "EP001"
    assert diagnostic.severity.label == "error"
    assert diagnostic.message == "parse error: unexpected End in Declarator"

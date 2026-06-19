from session import open_workspace


def test_check_accepts_valid_module() -> None:
    fixture = open_workspace(
        [
            ("destack.json", '{"name":"@test/app"}'),
            ("src/index.ds", "export const value = 1;"),
        ],
    )
    workspace = fixture.workspace
    revision = workspace.revision()
    module = workspace.module("src/index.ds")
    profile = workspace.profile(revision, module, "js")

    # check the module under the selected target profile
    output = workspace.check(revision, module, profile)

    assert output.diagnostics == []


def test_check_reports_missing_type_annotation() -> None:
    fixture = open_workspace(
        [
            ("destack.json", '{"name":"@test/app"}'),
            ("src/index.ds", "const values = [];"),
        ],
    )
    workspace = fixture.workspace
    revision = workspace.revision()
    module = workspace.module("src/index.ds")
    profile = workspace.profile(revision, module, "js")

    # check the module and read its type diagnostics
    output = workspace.check(revision, module, profile)

    assert len(output.diagnostics) == 2
    assert output.diagnostics[0].code == "EC101"
    assert output.diagnostics[0].severity.label == "error"
    assert output.diagnostics[0].message == "missing type annotation"
    assert output.diagnostics[1].code == "EC101"
    assert output.diagnostics[1].severity.label == "error"
    assert output.diagnostics[1].message == "missing type annotation"

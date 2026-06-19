from destack import LintRequest, Scope

from session import open_workspace


def test_lint_accepts_module() -> None:
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

    # lint the loaded module profile
    output = workspace.lint(revision, LintRequest(Scope.module(module, profile)))

    assert output.diagnostics == []

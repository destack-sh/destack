from destack import Document, FormatRequest

from session import open_workspace


def test_format_source() -> None:
    fixture = open_workspace(
        [
            ("destack.json", '{"name":"@test/app"}'),
        ],
    )
    workspace = fixture.workspace
    request = FormatRequest(Document.text("src/index.ds", "export  const value=1;"))

    # format ad hoc source text
    output = workspace.format(workspace.revision(), request)

    assert output.text == "export const value = 1;\n"

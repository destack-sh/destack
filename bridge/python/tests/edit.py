from destack import Edit

from session import open_workspace


def test_edit_source() -> None:
    fixture = open_workspace(
        [
            ("destack.json", '{"name":"@test/app"}'),
            ("src/index.ds", "export const value = 1;"),
        ],
    )
    workspace = fixture.workspace

    commit = workspace.edit(
        [
            Edit.set_text("src/next.ds", "export const next = 2;"),
        ],
    )

    assert commit.before.id != commit.after.id
    assert [file.path for file in workspace.files()] == [
        "destack.json",
        "src/index.ds",
        "src/next.ds",
    ]

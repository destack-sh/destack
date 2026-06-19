from session import open_workspace


def test_open_memory_source() -> None:
    fixture = open_workspace(
        [
            ("destack.json", '{"name":"@test/app"}'),
            ("src/index.ds", "export const value = 1;"),
        ],
    )
    workspace = fixture.workspace

    assert [file.path for file in workspace.files()] == [
        "destack.json",
        "src/index.ds",
    ]

from destack import ArtifactKey

from session import open_workspace


def test_content_reads_source_dependency() -> None:
    fixture = open_workspace(
        [
            ("destack.json", '{"name":"@test/app"}'),
            ("src/index.ds", "export const value = 1;"),
        ],
    )
    workspace = fixture.workspace
    revision = workspace.revision()
    module = workspace.module("src/index.ds")
    key = ArtifactKey.dir_parsed(module.id)

    # require the parsed artifact that records source content dependencies
    record = workspace.artifact_record(revision, key)

    # read the exact file content dependency through the public content API
    file_contents = [
        dependency.dependency.content
        for dependency in record.dependencies
        if dependency.kind == "source" and dependency.dependency.kind == "fileContent"
    ]

    assert len(file_contents) == 1
    assert workspace.text(file_contents[0]) == "export const value = 1;"

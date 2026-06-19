from destack import BuildRequest

from session import open_workspace


def test_build_links_js_bundle() -> None:
    fixture = open_workspace(
        [
            ("destack.json", '{"name":"@test/app"}'),
            ("src/index.ds", "export const value = 1;"),
        ],
    )
    workspace = fixture.workspace
    revision = workspace.revision()
    module = workspace.module("src/index.ds")
    target = workspace.target(revision, module.id.package, "js")

    # build the package target through the public bridge
    output = workspace.build(revision, BuildRequest.target(target))
    bundle = output.bundle
    assert bundle is not None
    file = bundle.files[0]

    assert output.kind == "bundle"
    assert bundle.emit.label == "js"
    assert bundle.mode.label == "singleFile"
    assert len(bundle.files) == 1
    assert file.section.label == "module"
    assert fixture.relative_uri(file.uri) == "dist/js.js"
    assert file.file_type.label == "javaScript"
    assert file.source is None

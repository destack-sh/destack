from __future__ import annotations

from pathlib import Path

from destack import open_workspace


def test_check_local_workspace(tmp_path: Path) -> None:
    """Check one local workspace through the native bridge."""

    root = tmp_path
    source = root / "src"
    source.mkdir()

    # write a minimal source tree for the local workspace server
    (root / "destack.json").write_text('{"name":"@test/app"}', encoding="utf-8")
    (source / "index.ds").write_text("export const value = 1;\n", encoding="utf-8")

    # drive the same workspace facade used for remote protocol clients
    workspace = open_workspace(workspace=str(root))
    output = workspace.check()
    workspace.close()

    assert output.diagnostics == []

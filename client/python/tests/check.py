from __future__ import annotations

import pytest

from workspace import workspace_modes

WORKSPACE_MODES = workspace_modes()


@pytest.mark.parametrize(
    ("_mode", "open_workspace"),
    WORKSPACE_MODES,
    ids=[mode for mode, _open_workspace in WORKSPACE_MODES],
)
def test_check_accepts_clean_workspace(_mode: str, open_workspace) -> None:
    """Check one clean workspace."""

    with open_workspace(
        config={
            "name": "@test/app",
        },
        files={},
    ) as workspace:
        # run the real check command through the opened workspace
        output = workspace.check(
            {
                "inputs": [
                    {
                        "kind": "inline",
                        "name": "input.ds",
                        "content": "export const value = 1;\n",
                        "file_type": "destack",
                    }
                ],
            }
        )

    assert output.success is True
    assert output.diagnostics == []

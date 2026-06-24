from __future__ import annotations

import pytest

from workspace import workspace_modes

WORKSPACE_MODES = workspace_modes()


@pytest.mark.parametrize(
    ("_mode", "open_workspace"),
    WORKSPACE_MODES,
    ids=[mode for mode, _open_workspace in WORKSPACE_MODES],
)
def test_format_returns_formatted_content(_mode: str, open_workspace) -> None:
    """Format one stored content value."""

    with open_workspace(
        config={
            "name": "@test/app",
        },
        files={},
    ) as workspace:
        # store content through the same workspace used by the formatter
        content = workspace.store("export  const value=1;\n")

        # run the real format command through the opened workspace
        output = workspace.format(
            {
                "source": {
                    "content": content,
                    "file_type": "destack",
                    "name": "input.ds",
                }
            }
        )

    assert output.success is True
    assert output.data.formatted == "export const value = 1;\n"

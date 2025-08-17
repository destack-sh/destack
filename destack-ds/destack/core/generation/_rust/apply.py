import os
import subprocess
from pathlib import Path
from typing import assert_never

from ...local import _console
from .core import local_path_to_raw_path
from .diff import RustFileOperation, RustFileOperationType, diff_rust_files
from .generate import generate_files
from .scan import scan_files


def regenerate(log_skip: bool = False):
    """Regenerate the entire Rust library / runtime."""
    from destack import SCHEMA

    old_rust_files = scan_files()
    new_rust_files = generate_files(SCHEMA)

    # diff
    ops = diff_rust_files(old_rust_files, new_rust_files)
    for op in ops:
        execute_operation(op, log_skip=log_skip)

    # format
    rs_dir = Path("..").absolute()
    subprocess.run(["cargo", "fmt"], cwd=rs_dir)
    subprocess.run(["cargo", "fix", "--allow-dirty"], cwd=rs_dir)


_OPERATION_STRING_LENGTH = max(len(t.upper()) for t in RustFileOperationType)

COLOR_BY_OPERATION_TYPE: dict[RustFileOperationType, _console.Color] = {
    RustFileOperationType.REPLACE: "blue",
    RustFileOperationType.PATCH: "yellow",
    RustFileOperationType.REMOVE: "red",
    RustFileOperationType.ADD: "green",
    RustFileOperationType.WARN: "bright_yellow",
    RustFileOperationType.SKIP: "dim",
}


def _render_operation_type(ty: RustFileOperationType) -> str:
    return _console.color(ty.upper().ljust(_OPERATION_STRING_LENGTH), COLOR_BY_OPERATION_TYPE[ty])


def execute_operation(operation: RustFileOperation, *, log_skip: bool):
    """Execute a Rust file operation."""

    path = local_path_to_raw_path(operation.local_path)

    # replace
    if operation.type == RustFileOperationType.REPLACE:
        assert operation.combined_content is not None, (
            f"REPLACE operation has no content: {operation!r}"
        )
        _console.info(f"{_render_operation_type(operation.type)}  {operation.local_path}")
        path.write_text(operation.combined_content)

    # patch
    elif operation.type == RustFileOperationType.PATCH:
        assert operation.combined_content is not None, (
            f"PATCH operation has no content: {operation!r}"
        )
        _console.info(f"{_render_operation_type(operation.type)}  {operation.local_path}")
        path.write_text(operation.combined_content)

    # remove
    elif operation.type == RustFileOperationType.REMOVE:
        _console.info(f"{_render_operation_type(operation.type)}  {operation.local_path}")
        path.unlink()

    # add
    elif operation.type == RustFileOperationType.ADD:
        assert operation.combined_content is not None, (
            f"ADD operation has no content: {operation!r}"
        )
        _console.info(f"{_render_operation_type(operation.type)}  {operation.local_path}")
        os.makedirs(path.parent, exist_ok=True)
        path.write_text(operation.combined_content)

    # warn
    elif operation.type == RustFileOperationType.WARN:
        _console.info(f"{_render_operation_type(operation.type)}  {operation.local_path}")

    # skip
    elif operation.type == RustFileOperationType.SKIP:
        if log_skip:
            _console.info(f"{_render_operation_type(operation.type)}  {operation.local_path}")

    else:
        assert_never(operation.type)

import os
from typing import assert_never

from .core import local_path_to_raw_path
from .diff import RustFileOperation, RustFileOperationType, diff_rust_files
from .generate import generate_files
from .scan import scan_files


def regenerate():
    """Regenerate the entire Rust library / runtime."""
    from destack import SCHEMA

    old_rust_files = scan_files()
    new_rust_files = generate_files(SCHEMA)

    ops = diff_rust_files(old_rust_files, new_rust_files)
    for op in ops:
        execute_operation(op)


def execute_operation(operation: RustFileOperation):
    """Execute a Rust file operation."""

    path = local_path_to_raw_path(operation.local_path)

    if (
        operation.type == RustFileOperationType.REPLACE
        or operation.type == RustFileOperationType.PATCH
    ):
        # nocheckin: destructive rust ops
        assert operation.combined_content is not None, (
            f"REPLACE or PATCH operation has no content: {operation!r}"
        )
        # path.write_text(operation.combined_content)
    elif operation.type == RustFileOperationType.REMOVE:
        # path.unlink()
        pass
    elif operation.type == RustFileOperationType.ADD:
        assert operation.combined_content is not None, (
            f"ADD operation has no content: {operation!r}"
        )
        os.makedirs(path.parent, exist_ok=True)
        path.write_text(operation.combined_content)
    elif operation.type == RustFileOperationType.WARN:
        pass  # noop
    else:
        assert_never(operation.type)

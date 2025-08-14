from dataclasses import dataclass
from enum import StrEnum

from .core import RustFile


class RustFileOperationType(StrEnum):
    """The type of operation on a Rust file."""

    REPLACE = "replace"  # replace fully
    PATCH = "patch"  # patch in place
    REMOVE = "remove"
    ADD = "add"


@dataclass(slots=True)
class RustFileOperation:
    """An operation on a Rust file."""

    type: RustFileOperationType
    normalized_path: str
    raw_path: str
    old_file: RustFile | None  # for REPLACE, PATCH and REMOVE
    new_file: RustFile | None  # for REPLACE, PATCH and ADD
    combined_content: str | None  # for REPLACE, PATCH and ADD


def diff_rust_files(
    old_files: dict[str, RustFile], new_files: dict[str, RustFile]
) -> list[RustFileOperation]:
    """Diff two sets of Rust files."""
    raise NotImplementedError

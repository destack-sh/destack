# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

import destack._generated.source.file
import destack._generated.source.span

if TYPE_CHECKING:
    from destack._generated.source.file import (
        FileId,
    )

    from destack._generated.source.span import (
        Span,
    )

@dataclass(frozen=True, slots=True)
class Patch:
    """One source patch crossing bridge boundaries."""

    """Source span to replace."""
    span: Span
    """Patch text."""
    new_text: str

def encode_patch(writer: Writer, value: Patch) -> None: ...
def decode_patch(reader: Reader) -> Patch: ...

@dataclass(frozen=True, slots=True)
class FilePatch:
    """Patches for a single file."""

    """Edited file."""
    file: FileId
    """Source patches."""
    patches: Sequence[Patch]

def encode_file_patch(writer: Writer, value: FilePatch) -> None: ...
def decode_file_patch(reader: Reader) -> FilePatch: ...

@dataclass(frozen=True, slots=True)
class PatchSet:
    """Patches across multiple files."""

    """Per-file patches."""
    files: Sequence[FilePatch]

def encode_patch_set(writer: Writer, value: PatchSet) -> None: ...
def decode_patch_set(reader: Reader) -> PatchSet: ...

__all__ = [
    "Patch",
    "encode_patch",
    "decode_patch",
    "FilePatch",
    "encode_file_patch",
    "decode_file_patch",
    "PatchSet",
    "encode_patch_set",
    "decode_patch_set",
]

from dataclasses import dataclass
from enum import StrEnum


class RustItemMode(StrEnum):
    OWNED = "owned"
    PARTIAL = "partial"
    STUB = "stub"


class RustItemScope(StrEnum):
    BLOCK = "block"
    LINE = "line"


@dataclass(slots=True)
class RustItem:
    object_key: str  # like 'Vector2'
    inner_key: str  # like 'struct', 'PartialEq'
    mode: RustItemMode
    scope: RustItemScope
    children: list["RustItem"]
    content: str  # the entire inner content without leading indentation


@dataclass(slots=True)
class RustImport:
    path: str  # like 'core::fmt'
    imports: list[str]  # like '["Add", "AddAssign", ...]'


@dataclass(slots=True)
class RustFile:
    path: str
    items: list[RustItem]

import dataclasses
from dataclasses import dataclass
from enum import StrEnum


class RustItemKind(StrEnum):
    CUSTOM = "custom"
    GENERATED = "generated"
    PARTIAL = "partial"


class RustItemScope(StrEnum):
    BLOCK = "block"
    LINE = "line"


@dataclass(slots=True)
class RustItem:
    """
    An individual 'item' in a Rust file.
    May be generated, partially generated or fully handwritten.
    """

    children: list["RustItem"]
    content: str  # the entire inner content without leading indentation


@dataclass(slots=True)
class RustCustomItem(RustItem):
    pass


@dataclass(slots=True)
class RustManagedItem(RustItem):
    object_key: str  # like 'Vector2'
    inner_key: str  # like 'struct', 'PartialEq', 'Add:Vector2'
    kind: RustItemKind
    scope: RustItemScope
    _key: str = dataclasses.field(init=False)

    def __post_init__(self):
        self._key = self.object_key + "." + self.inner_key


@dataclass(slots=True)
class RustAttribute:
    """A raw module-level attribute like `#![allow(...)]`."""

    content: str


@dataclass(slots=True)
class RustMod(RustItem):
    """A module declaration like `mod foo;` or `pub mod foo;`."""

    name: str
    is_public: bool


@dataclass(slots=True)
class RustImport:
    """An import at the top of the file."""

    path: str  # like 'core::fmt'
    imports: list[str]  # like '["Add", "AddAssign", ...]'
    is_internal: bool  # whether this import referes to the crate itself
    is_public: bool  # whether the import is declared with a public visibility
    is_glob: bool  # whether the import uses '*'


@dataclass(slots=True)
class RustFile:
    """A specific RustFile."""

    path: str
    items: list[RustItem]
    module_comment: str
    module_attributes: list[RustAttribute]
    imports: list["RustImport"] | None = None
    mods: list["RustMod"] | None = None

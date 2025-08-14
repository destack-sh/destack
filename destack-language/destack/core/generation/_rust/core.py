import dataclasses
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

DESTACK_RS_PATH = Path("../destack-rs/destack/src")


class RustGenerationType(StrEnum):
    """What type of generation."""

    """Fully custom part."""
    CUSTOM = "custom"
    """Fully managed part."""
    GENERATED = "generated"
    """Partially custom, partially managed."""
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
    type: RustGenerationType
    scope: RustItemScope
    _key: str = dataclasses.field(init=False)

    def __post_init__(self):
        self._key = self.object_key + "." + self.inner_key


@dataclass(slots=True)
class RustAttribute:
    """
    A raw module-level attribute like `#![allow(...)]`.
    Example: `#![destack::generated(vector, file)]`
    """

    content: str


@dataclass(slots=True)
class RustMod(RustItem):
    """A module declaration like `mod foo;` or `pub mod foo;`."""

    name: str
    is_public: bool


@dataclass(slots=True)
class RustImport:
    """An import at the top of the file."""

    """"""
    rust_path: str  # like 'core::fmt'
    imports: list[str]  # like '["Add", "AddAssign", ...]'
    is_internal: bool  # whether this import referes to the crate itself
    is_public: bool  # whether the import is declared with a public visibility
    is_glob: bool  # whether the import uses '*'


@dataclass(slots=True)
class RustFile:
    """A specific RustFile."""

    """The type of RustFile (set manually or determined via crate attributes)."""
    type: RustGenerationType
    """Raw relative path like destack/src/simulation/geometry/vector.rs"""
    raw_path: str
    """Normalized path like `destack.simulation.geometry.vector.Vector2`"""
    normalized_path: str
    """All items in the file (managed and custom)."""
    items: list[RustItem]
    """Top level comment."""
    comment: str
    """Top level attributes."""
    attributes: list[RustAttribute]
    """Top level imports."""
    imports: list["RustImport"] | None = None
    """Declared modules."""
    mods: list["RustMod"] | None = None
    """Whether this is the mod.rs file."""
    is_mod_rs: bool = False

import dataclasses
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

DESTACK_RS_PATH = Path("../destack-rs/destack/src")


def to_normalized_path(raw_path: Path) -> str:
    """Normalize a path to a Rust file."""
    normalized_path = str(raw_path.relative_to(DESTACK_RS_PATH).with_suffix(""))
    normalized_path = normalized_path.replace("/", ".")
    return "destack." + normalized_path


def to_raw_path(normalized_path: str) -> Path:
    """Convert a normalized path to a raw path."""
    return DESTACK_RS_PATH / normalized_path.replace("destack.", "src.").replace(".", "/")


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


def _render_to_string(file: RustFile) -> str:
    """
    Render the file as a simple canonical string.

    This is not a formatter; it is only for roundtrip structural tests.
    """

    parts: list[str] = []
    if file.comment:
        parts.append(file.comment.strip())
    if file.attributes:
        attrs_block = "\n".join(attr.content.strip() for attr in file.attributes)
        parts.append(attrs_block)
    rendered_items = []
    for item in file.items:
        if isinstance(item, RustManagedItem):
            attr = f"#[destack::{item.type}({item.object_key}, {item.inner_key}, {item.scope})]"
            rendered_items.append(f"{attr}\n{item.content}".strip())
        elif isinstance(item, RustMod):
            # check if it's a simple declaration or a nested module
            if item.children:
                # nested module with content
                rendered_items.append(item.content.strip())
            else:
                # simple module declaration
                prefix = "pub mod" if item.is_public else "mod"
                rendered_items.append(f"{prefix} {item.name};")
        else:
            rendered_items.append(item.content.strip())
    parts.append("\n\n".join(s for s in rendered_items if s))
    return "\n\n".join(s for s in parts if s)

import dataclasses
from collections.abc import Sequence
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

DESTACK_RS_PATH = Path("../destack-rs")
DESTACK_RS_SRC_PATH = DESTACK_RS_PATH / "destack" / "src"
DESTACK_RS_GEN_POSTFIX = "_gen"


def raw_path_to_source_path(raw_path: Path) -> str:
    """
    Convert a raw path to a source path.
    Returns: like `simulation.geometry.vector`
    """
    source_path = str(raw_path.relative_to(DESTACK_RS_SRC_PATH).with_suffix(""))
    source_path = source_path.replace("/", ".")
    return "destack." + source_path


def raw_path_to_local_path(raw_path: Path) -> str:
    """
    Convert a raw path to a local path.
    Trim common DESTACK_RS_PATH prefix and remove postfix.
    Returns: like `simulation/geometry/vector.rs` or `simulation/geometry/_gen/vector_gen.rs`
    """
    base_path = raw_path.relative_to(DESTACK_RS_SRC_PATH).with_suffix("")
    return str(base_path) + ".rs"


def source_path_to_local_path(source_path: str, *, is_gen: bool) -> str:
    """
    Convert a source path to a local path.
    Returns:
       `simulation/geometry/vector.rs` (is_gen=False)
       `simulation/geometry/_gen/vector_gen.rs` (is_gen=True)
    """
    local_path = source_path.replace("destack.", "").replace(".", "/")
    if is_gen:
        # put into _gen subfolder, add postfix
        local_path = (
            *local_path.split("/")[:-1],
            DESTACK_RS_GEN_POSTFIX,
            local_path.split("/")[-1] + DESTACK_RS_GEN_POSTFIX,
        )
        return "/".join(local_path) + ".rs"
    else:
        return str(local_path) + ".rs"


def local_path_to_raw_path(local_path: str) -> Path:
    """
    Convert a local path to a raw path.
    Returns: actual Path like `destack/src/simulation/geometry/vector.rs`
    """
    return DESTACK_RS_SRC_PATH / local_path


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
    outer_content: str  # the entire content without leading indentation
    inner_content: str  # the entire inner content without leading indentation
    # inner_content strips the block wrapper (like struct Vector { ... inner content ... })
    # for other items outer_content == inner_content


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
    is_inline: bool


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
    """Raw local path like destack/src/simulation/geometry/vector.rs"""
    local_path: str
    """Normalized source path like `destack.simulation.geometry.vector.Vector2`"""
    source_path: str
    """All items in the file (managed and custom)."""
    items: Sequence[RustItem]
    """Top level comment."""
    comment: str
    """Top level attributes."""
    attributes: Sequence[RustAttribute]
    """Top level imports."""
    imports: Sequence["RustImport"] | None = None
    """Declared modules."""
    mods: Sequence["RustMod"] | None = None
    """Whether this is the mod.rs file."""
    is_mod_rs: bool = False


def render_rust_destack_attribute(item: RustManagedItem) -> str:
    """Render a destack attribute."""
    return f"#[destack::{item.type}({item.object_key}, {item.inner_key}, {item.scope})]"


def render_rust_mod(mod: RustMod) -> str:
    """Render a module declaration."""
    if mod.is_inline:
        return mod.outer_content
    else:
        # just a declaration
        mod_str = f"mod {mod.name};"
        if mod.is_public:
            mod_str = f"pub {mod_str}"
        return mod_str


def render_rust_import(imp: RustImport) -> str:
    """Render an import."""
    if imp.is_glob:
        imp_str = f"use {imp.rust_path}::*;"
    elif imp.imports:
        imp_str = f"use {imp.rust_path}::{{{', '.join(imp.imports)}}};"
    else:
        imp_str = f"use {imp.rust_path};"
    if imp.is_public:
        imp_str = f"pub {imp_str}"
    return imp_str


def render_rust_file(file: RustFile) -> str:
    """
    Render the file as a simple canonical string.
    """
    parts: list[str] = []

    # header
    if file.comment:
        parts.append(file.comment.strip())
    if file.attributes:
        attrs_block = "\n".join(attr.content.strip() for attr in file.attributes)
        parts.append(attrs_block)
    if file.imports:
        imports_block = "\n".join(render_rust_import(imp) for imp in file.imports)
        parts.append(imports_block)

    # body
    rendered_items = []
    for item in file.items:
        if isinstance(item, RustManagedItem):
            attr = render_rust_destack_attribute(item)
            rendered_items.append(f"{attr}\n{item.outer_content}".strip())
        elif isinstance(item, RustMod):
            rendered_items.append(render_rust_mod(item))
        else:
            rendered_items.append(item.outer_content.strip())
    parts.append("\n\n".join(s for s in rendered_items if s))

    return "\n\n".join(s for s in parts if s)

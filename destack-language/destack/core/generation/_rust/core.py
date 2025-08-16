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


def local_path_to_source_path(local_path: str) -> str:
    """
    Convert a local path to a source path.
    Returns: like `simulation.geometry.vector`
    """
    source_path = local_path.replace("/", ".")
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


class RustManagedType(StrEnum):
    """What type of generation."""

    """Fully generated part."""
    GENERATED = "generated"
    """Partially custom, partially generated."""
    PARTIAL = "partial"
    """Fully custom part."""
    CUSTOM = "custom"


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
    type: RustManagedType
    scope: RustItemScope
    dependencies: Sequence[str] | None = None  # like ['MouseButtonType', 'TextSpan']
    _key: str = dataclasses.field(init=False)

    def __post_init__(self):
        if self.inner_key:
            self._key = self.object_key + "." + self.inner_key
        else:
            self._key = self.object_key


@dataclass(slots=True)
class RustAttribute:
    """
    A raw module-level attribute like `#![allow(...)]`.
    Example: `#![destack::generated(vector, file)]`
    """

    content: str


class RustVisibility(StrEnum):
    """The visibility of a Rust item."""

    PUBLIC = "pub"
    CRATE = "pub(crate)"
    SUPER = "pub(super)"
    PRIVATE = "private"
    # we don't use pub(in ...)


@dataclass(slots=True)
class RustModDeclaration:
    """A module declaration like `mod foo;` or `pub mod foo;`."""

    name: str
    is_public: bool


@dataclass(slots=True)
class RustImport:
    """An import at the top of the file."""

    source: str  # like 'core (up to the imported items, but excluding them)
    imports: list[str]  # like '["fmt", "Add", "AddAssign", ...]'
    is_glob: bool  # whether the import uses '*'
    is_internal: bool  # whether this import referes to the crate itself
    is_public: bool  # whether the import is declared with 'pub'


@dataclass(slots=True)
class RustFile:
    """A specific RustFile."""

    """The type of RustFile (set manually or determined via crate attributes)."""
    type: RustManagedType
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
    mods: Sequence["RustModDeclaration"] | None = None
    """Whether this is the mod.rs file."""
    is_mod_rs: bool = False
    """Raw source content."""
    raw_content: str | None = None


def render_rust_destack_attribute(item: RustManagedItem) -> str:
    """Render a destack attribute."""
    return f"#[destack::{item.type}({item.object_key}, {item.inner_key or '-'}, {item.scope})]"


def render_rust_mod(mod: RustModDeclaration) -> str:
    """Render a module declaration."""
    # just a declaration
    mod_str = f"mod {ecsape_rust_identifier(mod.name)};"
    if mod.is_public:
        mod_str = f"pub {mod_str}"
    return mod_str


def render_rust_import(imp: RustImport) -> str:
    """Render an import."""
    escaped_path = ecsape_rust_identifier(imp.source, keep=("crate",))
    if imp.is_glob:
        use_stmt = f"use {escaped_path}::*;"
    elif imp.imports:
        escaped_imports = [ecsape_rust_identifier(item) for item in imp.imports]
        if len(escaped_imports) == 1:
            use_stmt = f"use {escaped_path}::{escaped_imports[0]};"
        else:
            imports_list = ", ".join(escaped_imports)
            use_stmt = f"use {escaped_path}::{{{imports_list}}};"
    else:
        raise ValueError(f"no imports for: {imp!r}")
    if imp.is_public:
        use_stmt = f"pub {use_stmt}"
    return use_stmt


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
    if file.mods:
        mods_block = "\n".join(render_rust_mod(mod) for mod in file.mods)
        parts.append(mods_block)

    # body
    rendered_items = []
    for item in file.items:
        if isinstance(item, RustManagedItem):
            attr = render_rust_destack_attribute(item)
            rendered_items.append(f"{attr}\n{item.outer_content}".strip())
        elif isinstance(item, RustCustomItem):
            rendered_items.append(item.outer_content.strip())
        else:
            raise ValueError(f"unexpected item: {item!r}")
    parts.append("\n\n".join(s for s in rendered_items if s))

    return "\n\n".join(s for s in parts if s)


RUST_RESERVED_KEYWORDS = {
    "as",
    "async",
    "await",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    # weak keywords (contextual)
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "override",
    "priv",
    "typeof",
    "unsized",
    "virtual",
    "yield",
}


def ecsape_rust_identifier(identifier: str, *, keep: Sequence[str] | None = None) -> str:
    """Escape an identifier (prefix keywords with 'r#')"""
    if "::" in identifier:
        identifier_parts = identifier.split("::")
        if identifier.startswith("crate::"):
            return "crate::" + "::".join(
                ecsape_rust_identifier(part) for part in identifier_parts[1:]
            )
        else:
            return "::".join(ecsape_rust_identifier(part) for part in identifier_parts)
    elif identifier in RUST_RESERVED_KEYWORDS and (keep is None or identifier not in keep):
        return f"r#{identifier}"
    else:
        return identifier


def unescape_rust_identifier(identifier: str) -> str:
    """Remove r# prefix from escaped identifiers."""
    if "::" in identifier:
        identifier_parts = identifier.split("::")
        if identifier.startswith("crate::"):
            return "crate::" + "::".join(
                unescape_rust_identifier(part) for part in identifier_parts[1:]
            )
        else:
            return "::".join(unescape_rust_identifier(part) for part in identifier_parts)
    elif identifier.startswith("r#"):
        return identifier[2:]
    else:
        return identifier

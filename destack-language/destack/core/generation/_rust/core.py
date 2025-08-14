from dataclasses import dataclass
from enum import StrEnum


class RustItemKind(StrEnum):
    CUSTOM = "custom"
    SYNTHETIC = "synthetic"
    PARTIAL = "partial"
    STUB = "stub"


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

    def render_to_string(self) -> str:
        """Render the file as a simple canonical string.

        This is not a formatter; it is only for roundtrip structural tests.
        """
        rendered_items = []
        for item in self.items:
            if isinstance(item, RustManagedItem):
                attr = f"#[destack::{item.kind}({item.object_key}, {item.inner_key}, {item.scope})]"
                rendered_items.append(f"{attr}\n{item.content}".strip())
            elif isinstance(item, RustMod):
                prefix = "pub mod" if item.is_public else "mod"
                rendered_items.append(f"{prefix} {item.name};")
            else:
                rendered_items.append(item.content.strip())
        return "\n\n".join(s for s in rendered_items if s)

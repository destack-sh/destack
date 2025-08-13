from __future__ import annotations

from collections.abc import Sequence

from destack.core.generation._rust.parse import (
    RustItemMode,
    RustItemScope,
    parse_rust_file,
    parse_rust_imports,
    parse_rust_items,
)

FILE_CONTENT: str = """\
use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[destack::generated(Vector2, struct, block)]
#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[destack::generated(Vector2, PartialEq, block)]
impl PartialEq for Vector2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

#[destack::generated(Vector2, Default, block)]
impl Default for Vector2 {
    fn default() -> Self {
        Self::ZERO
    }
}

#[destack::generated(Vector2, Display, block)]
impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[destack::partial(Vector2, impl, block)]
impl Vector2 {
    #[destack::generated(Vector2, ZERO, line)]
    pub const ZERO: Vector2 = Vector2 { x: 0.0, y: 0.0 };

    #[destack::generated(Vector2, ONE, line)]
    pub const ONE: Vector2 = Vector2 { x: 1.0, y: 1.0 };

    #[destack::generated(Vector2, X_AXIS, line)]
    pub const X_AXIS: Vector2 = Vector2 { x: 1.0, y: 0.0 };

    #[destack::generated(Vector2, Y_AXIS, line)]
    pub const Y_AXIS: Vector2 = Vector2 { x: 0.0, y: 1.0 };

    #[destack::stub(Vector2, x, function_stub)]
    #[inline]
    /// Return the x component.
    pub const fn x(&self) -> f32 {
        self.x
    }
}

#[destack::partial(Vector2, Add, block)]
// Arithmetic ops with another Vector2
impl Add for Vector2 {
    type Output = Vector2;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Vector2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
"""


def _assert_has_top_level_items(items: Sequence) -> None:
    # 6 top-level items: struct, PartialEq, Default, Display, impl Vector2 (partial), Add (partial)
    assert len(items) == 6
    assert items[0].object_key == "Vector2" and items[0].inner_key == "struct"
    assert items[0].mode == RustItemMode.GENERATED and items[0].scope == RustItemScope.BLOCK
    assert items[1].inner_key == "PartialEq" and items[1].mode == RustItemMode.GENERATED
    assert items[2].inner_key == "Default" and items[2].mode == RustItemMode.GENERATED
    assert items[3].inner_key == "Display" and items[3].mode == RustItemMode.GENERATED
    assert items[4].inner_key == "impl" and items[4].mode == RustItemMode.PARTIAL
    assert items[5].inner_key == "Add" and items[5].mode == RustItemMode.PARTIAL


def test_parse_rust_imports() -> None:
    """Parse top-level use imports into structured paths and lists."""
    imports = parse_rust_imports(FILE_CONTENT)
    assert len(imports) == 2
    assert imports[0].path == "core::fmt" and imports[0].imports == []
    assert imports[1].path == "core::ops"
    assert set(imports[1].imports) >= {
        "Add",
        "AddAssign",
        "Div",
        "DivAssign",
        "Mul",
        "MulAssign",
        "Neg",
        "Sub",
        "SubAssign",
    }


def test_parse_top_level_block_items() -> None:
    """Parse top-level block-scoped annotated items (struct and impls)."""
    items = parse_rust_items(FILE_CONTENT)
    _assert_has_top_level_items(items)
    # verify content headers are present
    assert items[0].content.splitlines()[0].startswith("pub struct Vector2 ")
    assert items[1].content.splitlines()[0].startswith("impl PartialEq for Vector2 ")
    assert items[2].content.splitlines()[0].startswith("impl Default for Vector2 ")
    assert items[3].content.splitlines()[0].startswith("impl fmt::Display for Vector2 ")


def test_parse_children_in_partial_impl_block() -> None:
    """Parse nested line and block items inside a partial impl block."""
    items = parse_rust_items(FILE_CONTENT)
    partial_impl = items[4]
    assert partial_impl.inner_key == "impl" and partial_impl.scope == RustItemScope.BLOCK
    children = partial_impl.children
    assert len(children) == 5
    # 4 generated line constants
    for idx, const_name in enumerate(["ZERO", "ONE", "X_AXIS", "Y_AXIS"]):
        child = children[idx]
        assert child.mode == RustItemMode.GENERATED
        assert child.scope == RustItemScope.LINE
        assert child.inner_key == const_name
        assert const_name in child.content
        assert child.content.endswith(";")
    # stubbed function is a block
    stub = children[4]
    assert stub.mode == RustItemMode.STUB
    assert stub.scope == RustItemScope.BLOCK
    assert stub.inner_key == "x"
    assert "fn x(&self) -> f32" in stub.content
    assert stub.content.strip().endswith("}")


def test_parse_full_file_wrapper() -> None:
    """Parse a full file into a RustFile wrapper with all items."""
    rf = parse_rust_file(FILE_CONTENT, path="/src/vector2.rs")
    assert rf.path.endswith("vector2.rs")
    _assert_has_top_level_items(rf.items)


def test_parse_full_source_string_end_to_end() -> None:
    """End-to-end: keep the full input and ensure we parse the expected count and order."""
    items = parse_rust_items(FILE_CONTENT)
    _assert_has_top_level_items(items)

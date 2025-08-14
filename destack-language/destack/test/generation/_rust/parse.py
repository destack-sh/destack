from collections.abc import Sequence

from destack.core.generation._rust.core import RustCustomItem, RustManagedItem
from destack.core.generation._rust.parse import (
    RustItemKind,
    RustItemScope,
    parse_rust_file,
    parse_rust_imports,
    parse_rust_items,
    parse_rust_mods,
)

FILE_1: str = """\
//! Module level comment.
//! 
//! More module level comment.

#![allow(clippy::large_enum_variant)]
#![cfg(test)]

use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[destack::synthetic(Vector2, struct, block)]
#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[destack::synthetic(Vector2, PartialEq, block)]
impl PartialEq for Vector2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

#[destack::synthetic(Vector2, Default, block)]
impl Default for Vector2 {
    fn default() -> Self {
        Self::ZERO
    }
}

#[destack::synthetic(Vector2, Display, block)]
impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[destack::partial(Vector2, impl, block)]
impl Vector2 {
    pub(crate) const INTERNAL_CONST: Vector2 = Vector2 { x: 111.0, y: 222.0 };

    #[destack::synthetic(Vector2, ZERO, line)]
    pub const ZERO: Vector2 = Vector2 { x: 0.0, y: 0.0 };

    #[destack::synthetic(Vector2, ONE, line)]
    pub const ONE: Vector2 = Vector2 { x: 1.0, y: 1.0 };

    pub(crate) const INTERNAL_CONST_2: Vector2 = Vector2 { x: 111.0, y: 222.0 };
    
    #[destack::synthetic(Vector2, X_AXIS, line)]
    pub const X_AXIS: Vector2 = Vector2 { x: 1.0, y: 0.0 };

    #[destack::synthetic(Vector2, Y_AXIS, line)]
    pub const Y_AXIS: Vector2 = Vector2 { x: 0.0, y: 1.0 };

    #[destack::stub(Vector2, x, function_stub)]
    #[inline]
    /// Return the x component.
    pub const fn x(&self) -> f32 {
        self.x
    }

    #[inline]
    fn custom_function(&mut self) {
        // test
    }
}

impl Vector2 {
    // more custom implementation
}

#[destack::partial(Vector2, Add:Vector2, block)]
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


FILE_2: str = """\
//! Module level documentation.

// example showing braces inside strings and comments should be ignored

#[destack::synthetic(Demo, struct, block)]
pub struct Demo {
    pub field: &'static str,
}

impl Demo {
    // this is a line comment with a brace { and another }
    /* block comment with nested { /* inner { } */ still comment } */
    pub const TEXT: &str = "braces in string { not a block } and escaped quote \" ok";
    pub const RAW: &str = r#"raw { string } with #"#;
}
"""


def _assert_has_top_level_items(items: Sequence) -> None:
    """Assert the 6 managed top-level items are present and in order.

    We filter out custom items and only check the managed sequence and kinds.
    """
    managed = [i for i in items if isinstance(i, RustManagedItem)]
    # 6 top-level managed items: struct, PartialEq, Default, Display, impl Vector2 (partial), Add (partial)
    assert len(managed) == 6
    assert managed[0].object_key == "Vector2" and managed[0].inner_key == "struct"
    assert managed[0].kind == RustItemKind.SYNTHETIC and managed[0].scope == RustItemScope.BLOCK
    assert managed[1].inner_key == "PartialEq" and managed[1].kind == RustItemKind.SYNTHETIC
    assert managed[2].inner_key == "Default" and managed[2].kind == RustItemKind.SYNTHETIC
    assert managed[3].inner_key == "Display" and managed[3].kind == RustItemKind.SYNTHETIC
    assert managed[4].inner_key == "impl" and managed[4].kind == RustItemKind.PARTIAL
    assert managed[5].inner_key == "Add:Vector2" and managed[5].kind == RustItemKind.PARTIAL


def test_parse_rust_imports() -> None:
    """Parse top-level use imports into structured paths and lists."""
    imports = parse_rust_imports(FILE_1)
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
    # internal flags
    assert imports[0].is_internal is False
    assert imports[1].is_internal is False


def test_parse_internal_rust_imports() -> None:
    """Detect internal import paths via crate/self/super prefixes."""
    src = """\
use crate::foo;
use self::bar::{Baz};
use super::qux;
use external::pkg::Thing;
pub use core::*;
"""
    imports = parse_rust_imports(src)
    assert len(imports) == 5
    assert imports[0].path == "crate::foo" and imports[0].is_internal is True
    assert (
        imports[1].path == "self::bar"
        and imports[1].imports == ["Baz"]
        and imports[1].is_internal is True
    )
    assert imports[2].path == "super::qux" and imports[2].is_internal is True
    assert imports[3].path == "external::pkg::Thing" and imports[3].is_internal is False
    assert imports[4].path == "core::*" or imports[4].path == "core::"  # path retains star
    assert imports[4].is_public is True and imports[4].is_glob is True


def test_parse_top_level_block_items() -> None:
    """Parse top-level block-scoped annotated items (struct and impls)."""
    items = parse_rust_items(FILE_1)
    _assert_has_top_level_items(items)
    # verify content headers are present
    managed = [i for i in items if isinstance(i, RustManagedItem)]
    assert managed[0].content.splitlines()[0].startswith("pub struct Vector2 ")
    assert managed[1].content.splitlines()[0].startswith("impl PartialEq for Vector2 ")
    assert managed[2].content.splitlines()[0].startswith("impl Default for Vector2 ")
    assert managed[3].content.splitlines()[0].startswith("impl fmt::Display for Vector2 ")

    # also ensure custom top-level impl is captured
    custom_top_level = [
        i for i in items if isinstance(i, RustCustomItem) and i.content.startswith("impl Vector2 ")
    ]
    assert len(custom_top_level) == 1


def test_parse_children_in_partial_impl_block() -> None:
    """Parse nested line and block items inside a partial impl block."""
    items = parse_rust_items(FILE_1)
    managed = [i for i in items if isinstance(i, RustManagedItem)]
    partial_impl = managed[4]
    assert partial_impl.inner_key == "impl" and partial_impl.scope == RustItemScope.BLOCK
    children = partial_impl.children
    # expect: 2 custom consts, 4 synthetic line consts, 1 stub block, 1 custom function block
    assert len(children) == 8
    # check the 4 generated line constants are present
    synthetic_lines = [
        c
        for c in children
        if isinstance(c, RustManagedItem)
        and c.kind == RustItemKind.SYNTHETIC
        and c.scope == RustItemScope.LINE
    ]
    assert {c.inner_key for c in synthetic_lines} == {"ZERO", "ONE", "X_AXIS", "Y_AXIS"}
    for c in synthetic_lines:
        assert c.content.endswith(";") and c.inner_key in c.content
    # stubbed function is a block
    stubs = [c for c in children if isinstance(c, RustManagedItem) and c.kind == RustItemKind.STUB]
    assert len(stubs) == 1
    stub = stubs[0]
    assert stub.scope == RustItemScope.BLOCK and stub.inner_key == "x"
    assert "fn x(&self) -> f32" in stub.content and stub.content.strip().endswith("}")
    # two custom constants captured
    custom_consts = [
        c for c in children if isinstance(c, RustCustomItem) and "INTERNAL_CONST" in c.content
    ]
    assert len(custom_consts) == 2
    # custom function at the bottom captured as a custom block
    custom_funcs = [
        c for c in children if isinstance(c, RustCustomItem) and "fn custom_function" in c.content
    ]
    assert len(custom_funcs) == 1

    # ensure content order preserves relative placement
    order = [
        "INTERNAL_CONST",
        "ZERO",
        "ONE",
        "INTERNAL_CONST_2",
        "X_AXIS",
        "Y_AXIS",
        "x(&self)",
        "fn custom_function",
    ]
    flattened = "\n\n".join(c.content for c in children)
    last_pos = -1
    for token in order:
        pos = flattened.find(token)
        assert pos != -1
        assert pos > last_pos
        last_pos = pos


def test_parse_full_file_wrapper() -> None:
    """Parse a full file into a RustFile wrapper with all items."""
    rf = parse_rust_file(FILE_1, path="/src/vector2.rs")
    assert rf.path.endswith("vector2.rs")
    _assert_has_top_level_items(rf.items)
    # ensure top-level custom impl block is preserved
    top_level_custom_impls = [
        i
        for i in rf.items
        if isinstance(i, RustCustomItem) and i.content.startswith("impl Vector2 ")
    ]
    assert len(top_level_custom_impls) == 1


def test_parse_full_source_string_end_to_end() -> None:
    """End-to-end: keep the full input and ensure we parse the expected count and order."""
    items = parse_rust_items(FILE_1)
    _assert_has_top_level_items(items)


def test_collect_block_ignores_strings_and_comments() -> None:
    """Ensure braces inside strings/comments do not break block collection."""
    items = parse_rust_items(FILE_2)
    managed = [i for i in items if isinstance(i, RustManagedItem)]
    assert len(managed) == 1
    assert managed[0].inner_key == "struct"
    customs = [i for i in items if isinstance(i, RustCustomItem)]
    # the impl block should be a single custom block
    assert any(c.content.startswith("impl Demo ") for c in customs)


def test_parse_mods() -> None:
    """Parse mod declarations from source."""
    src = """\
// header
pub use core::*;
mod inner;

#[destack::synthetic(Thing, struct, block)]
pub struct Thing {
    pub a: i32,
}
"""
    mods = parse_rust_mods(src)
    assert len(mods) == 1
    assert mods[0].name == "inner" and mods[0].is_public is False

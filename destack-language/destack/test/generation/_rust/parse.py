from collections.abc import Sequence

import pytest

from destack.core.generation._rust.core import (
    RustCustomItem,
    RustManagedItem,
    RustVisibility,
    render_rust_file,
)
from destack.core.generation._rust.parse import (
    RustItemScope,
    RustManagedType,
    _parse_rust_imports,
    _parse_rust_items,
    _parse_rust_mod_declarations,
    parse_rust_file,
)

FILE_1: str = """\
//! Module level comment.
//!
//! More module level comment.

#![destack::partial(vector, file)]
#![allow(clippy::large_enum_variant)]
#![cfg(test)]

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
    pub const INTERNAL_CONST: Vector2 = Vector2 { x: 111.0, y: 222.0 };

    #[destack::generated(Vector2, ZERO, line)]
    pub const ZERO: Vector2 = Vector2 { x: 0.0, y: 0.0 };

    #[destack::generated(Vector2, ONE, line)]
    pub const ONE: Vector2 = Vector2 { x: 1.0, y: 1.0 };

    pub const INTERNAL_CONST_2: Vector2 = Vector2 { x: 111.0, y: 222.0 };
    
    #[destack::generated(Vector2, X_AXIS, line)]
    pub const X_AXIS: Vector2 = Vector2 { x: 1.0, y: 0.0 };

    #[destack::generated(Vector2, Y_AXIS, line)]
    pub const Y_AXIS: Vector2 = Vector2 { x: 0.0, y: 1.0 };

    #[destack::partial(Vector2, x, block)]
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

#![destack::partial(vector, file)]

// example showing braces inside strings and comments should be ignored

#[destack::generated(Demo, struct, block)]
pub struct Demo {
    pub field: &'static str,
}

impl Demo {
    // this is a line comment with a brace { and another }
    /* block comment with nested { /* inner { } */ still comment } */
    pub const TEXT: &str = "braces in string { not a block } and escaped quote \" ok";
    pub const RAW: &str = r#"raw { string } with #"#;
}

#[destack::partial(vector, vector2, block)]
fn vector2(x: f32, y: f32) -> Vector2 {
    Vector2 {
        x: x,
        y: y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frac() {
        assert_eq!(parse_us_maybe(b"").unwrap(), None);
    }
    
    #[test]
    fn test_another() {
        let x = Vector2 { x: 1.0, y: 2.0 };
        assert_eq!(x.x, 1.0);
    }
}
"""


FILES = {"FILE_1": FILE_1, "FILE_2": FILE_2}


def _assert_has_top_level_items(items: Sequence) -> None:
    """Assert the 6 managed top-level items are present and in order.

    We filter out custom items and only check the managed sequence and kinds.
    """
    managed = [i for i in items if isinstance(i, RustManagedItem)]
    # 6 top-level managed items: struct, PartialEq, Default, Display, impl Vector2 (partial), Add (partial)
    assert len(managed) == 6
    assert managed[0].object_key == "Vector2" and managed[0].inner_key == "struct"
    assert managed[0].type == RustManagedType.GENERATED and managed[0].scope == RustItemScope.BLOCK
    assert managed[1].inner_key == "PartialEq" and managed[1].type == RustManagedType.GENERATED
    assert managed[2].inner_key == "Default" and managed[2].type == RustManagedType.GENERATED
    assert managed[3].inner_key == "Display" and managed[3].type == RustManagedType.GENERATED
    assert managed[4].inner_key == "impl" and managed[4].type == RustManagedType.PARTIAL
    assert managed[5].inner_key == "Add:Vector2" and managed[5].type == RustManagedType.PARTIAL


def test_parse_rust_imports() -> None:
    """Detect internal import paths via crate/self/super prefixes."""
    src = """\
use crate::foo;
use self::bar::Baz;
use super::qux;
use external::pkg::Thing;
pub use core::*;
use std::collections::{HashMap, HashSet};
use crate::internal::module::{
    InternalStruct,
    AnotherStruct,
    ThirdStruct,
};
pub(crate) use alloc::vec::Vec;
pub(super) use crate::m::Inner;
"""
    imports, _ = _parse_rust_imports(src)
    assert len(imports) == 9

    # use crate::foo;
    assert imports[0].source == "crate"
    assert imports[0].is_internal is True
    assert imports[0].imports == ["foo"]
    assert imports[0].visibility == RustVisibility.PRIVATE

    # use self::bar::Baz;
    assert imports[1].source == "self::bar"
    assert imports[1].is_internal is True
    assert imports[1].imports == ["Baz"]
    assert imports[1].visibility == RustVisibility.PRIVATE

    # use super::qux;
    assert imports[2].source == "super"
    assert imports[2].is_internal is True
    assert imports[2].imports == ["qux"]
    assert imports[2].visibility == RustVisibility.PRIVATE

    # use external::pkg::Thing;
    assert imports[3].source == "external::pkg"
    assert imports[3].is_internal is False
    assert imports[3].imports == ["Thing"]
    assert imports[3].visibility == RustVisibility.PRIVATE

    # pub use core::*;
    assert imports[4].source == "core"
    assert imports[4].is_glob is True
    assert imports[4].imports == []  # should be empty, not ['*']
    assert imports[4].visibility == RustVisibility.PUBLIC

    # use std::collections::{HashMap, HashSet};
    assert imports[5].source == "std::collections"
    assert imports[5].is_internal is False
    assert set(imports[5].imports) == {"HashMap", "HashSet"}
    assert imports[5].visibility == RustVisibility.PRIVATE

    # use crate::internal::module::{
    #     InternalStruct,
    #     AnotherStruct,
    #     ThirdStruct
    # };
    assert imports[6].source == "crate::internal::module"
    assert imports[6].is_internal is True
    assert set(imports[6].imports) == {"InternalStruct", "AnotherStruct", "ThirdStruct"}
    assert imports[6].visibility == RustVisibility.PRIVATE

    # pub(crate) use alloc::vec::Vec;
    assert imports[7].source == "alloc::vec"
    assert imports[7].is_internal is False
    assert imports[7].imports == ["Vec"]
    assert imports[7].visibility == RustVisibility.CRATE

    # pub(super) use crate::m::Inner;
    assert imports[8].source == "crate::m"
    assert imports[8].is_internal is True
    assert imports[8].imports == ["Inner"]
    assert imports[8].visibility == RustVisibility.SUPER


def test_parse_block_items() -> None:
    """Parse top-level block-scoped annotated items (struct and impls)."""
    items = _parse_rust_items(FILE_1, parent=None, ignore_lines=())
    _assert_has_top_level_items(items)
    # verify content headers are present
    managed = [i for i in items if isinstance(i, RustManagedItem)]
    # managed[0] may include attribute prefix lines, so check header exists in content
    assert any(
        line.startswith("pub struct Vector2 ") for line in managed[0].outer_content.splitlines()
    )
    assert managed[1].outer_content.splitlines()[0].startswith("impl PartialEq for Vector2 ")
    assert managed[2].outer_content.splitlines()[0].startswith("impl Default for Vector2 ")
    assert managed[3].outer_content.splitlines()[0].startswith("impl fmt::Display for Vector2 ")

    # also ensure custom top-level impl is captured
    custom_top_level = [
        i
        for i in items
        if isinstance(i, RustCustomItem) and i.outer_content.startswith("impl Vector2 ")
    ]
    assert len(custom_top_level) == 1


def test_parse_children_in_partial_impl_block() -> None:
    """Parse nested line and block items inside a partial impl block."""
    items = _parse_rust_items(FILE_1, parent=None, ignore_lines=())
    managed = [i for i in items if isinstance(i, RustManagedItem)]
    partial_impl = managed[4]
    assert partial_impl.inner_key == "impl" and partial_impl.scope == RustItemScope.BLOCK
    children = partial_impl.children
    # expect: 2 custom consts, 4 generated line consts, 1 stub block, 1 custom function block
    assert len(children) == 8
    # check the 4 generated line constants are present
    generated_lines = [
        c
        for c in children
        if isinstance(c, RustManagedItem)
        and c.type == RustManagedType.GENERATED
        and c.scope == RustItemScope.LINE
    ]
    assert {c.inner_key for c in generated_lines} == {"ZERO", "ONE", "X_AXIS", "Y_AXIS"}
    for c in generated_lines:
        assert c.outer_content.endswith(";") and c.inner_key in c.outer_content
    # stubbed function is a block
    stubs = [
        c for c in children if isinstance(c, RustManagedItem) and c.type == RustManagedType.PARTIAL
    ]
    assert len(stubs) == 1
    stub = stubs[0]
    assert stub.scope == RustItemScope.BLOCK and stub.inner_key == "x"
    assert "fn x(&self) -> f32" in stub.outer_content and stub.outer_content.strip().endswith("}")
    # two custom constants captured
    custom_consts = [
        c for c in children if isinstance(c, RustCustomItem) and "INTERNAL_CONST" in c.outer_content
    ]
    assert len(custom_consts) == 2
    # custom function at the bottom captured as a custom block
    custom_funcs = [
        c
        for c in children
        if isinstance(c, RustCustomItem) and "fn custom_function" in c.outer_content
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
    flattened = "\n\n".join(c.outer_content for c in children)
    last_pos = -1
    for token in order:
        pos = flattened.find(token)
        assert pos != -1
        assert pos > last_pos
        last_pos = pos


def test_parse_full_file_wrapper() -> None:
    """Parse a full file into a RustFile wrapper with all items."""
    rf = parse_rust_file(
        source=FILE_1,
        source_path="simulation.geometry.vector",
        local_path="simulation/geometry/vector2.rs",
    )
    _assert_has_top_level_items(rf.items)
    # ensure top-level custom impl block is preserved
    top_level_custom_impls = [
        i
        for i in rf.items
        if isinstance(i, RustCustomItem) and i.outer_content.startswith("impl Vector2 ")
    ]
    assert len(top_level_custom_impls) == 1


def test_parse_full_source_string_end_to_end() -> None:
    """End-to-end: keep the full input and ensure we parse the expected count and order."""
    items = _parse_rust_items(FILE_1, parent=None, ignore_lines=())
    _assert_has_top_level_items(items)


def test_parse_free_block() -> None:
    """Parse a free function stub annotated at top level."""
    items = _parse_rust_items(FILE_2, parent=None, ignore_lines=())
    stubs = [
        i
        for i in items
        if isinstance(i, RustManagedItem)
        and i.type == RustManagedType.PARTIAL
        and i.inner_key == "vector2"
    ]
    assert len(stubs) == 1
    stub = stubs[0]
    assert stub.scope == RustItemScope.BLOCK
    assert "fn vector2(" in stub.outer_content
    assert stub.outer_content.strip().endswith("}")


def test_parse_mods() -> None:
    """Parse mod declarations from source."""
    src = """\
// header
pub use core::*;
mod inner;
pub mod public_mod;
pub(crate) mod crate_mod;
pub(super) mod super_mod;

#[destack::generated(Thing, struct, block)]
pub struct Thing {
    pub a: i32,
}
"""
    mods, _ = _parse_rust_mod_declarations(src)
    assert len(mods) == 4
    assert mods[0].name == "inner" and mods[0].visibility == RustVisibility.PRIVATE
    assert mods[1].name == "public_mod" and mods[1].visibility == RustVisibility.PUBLIC
    assert mods[2].name == "crate_mod" and mods[2].visibility == RustVisibility.CRATE
    assert mods[3].name == "super_mod" and mods[3].visibility == RustVisibility.SUPER


def test_parse_file_type() -> None:
    # explicit partial attribute -> PARTIAL
    file_content = """\
//! Module level documentation.

#![destack::partial(vector, file)]

//! Module level documentation.
"""
    file = parse_rust_file(source=file_content, source_path="", local_path="")
    assert file.type == RustManagedType.PARTIAL

    # explicit generated attribute -> GENERATED
    file_content = """\
//! Module level documentation.

#![destack::generated(vector, file)]

//! Module level documentation.
"""
    file = parse_rust_file(source=file_content, source_path="", local_path="")
    assert file.type == RustManagedType.GENERATED

    # no attributes -> CUSTOM
    file_content = """\
//! Module level documentation.
//! Module level documentation.
"""
    file = parse_rust_file(source=file_content, source_path="", local_path="")
    assert file.type == RustManagedType.CUSTOM


@pytest.mark.parametrize("file_name", ["FILE_1", "FILE_2"])
def test_roundtrip_parse(file_name: str) -> None:
    """Parse and render back to string; ensure structural equality and sanity."""
    file_str = FILES[file_name]
    parsed_file = parse_rust_file(
        source=file_str,
        source_path="simulation.geometry.vector",
        local_path="simulation/geometry/vector2.rs",
    )
    rendered_file_str = render_rust_file(parsed_file)
    # input and output should match (ignoring trailing spaces)
    file_str_clean = "\n".join(line.rstrip() for line in file_str.splitlines()).strip()
    rendered_file_str_clean = "\n".join(
        line.rstrip() for line in rendered_file_str.splitlines()
    ).strip()
    assert file_str_clean == rendered_file_str_clean

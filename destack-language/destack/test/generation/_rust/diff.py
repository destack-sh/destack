from destack.core.generation._rust.diff import RustFileOperationType, diff_rust_file
from destack.core.generation._rust.parse import parse_rust_file

# The 'old' file read from disk
FILE_OLD: str = """\
#![destack::partial(vector, file)]

#[destack::generated(Vector2, struct, block)]
#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[destack::generated(Vector2, Default, block)]
impl Default for Vector2 {
    fn default() -> Self {
        Self::ZERO
    }
}

#[destack::partial(Vector2, impl, block)]
impl Vector2 {
    pub const INTERNAL_CONST: Vector2 = Vector2 { x: 111.0, y: 222.0 };

    #[destack::generated(Vector2, ZERO, line)]
    pub const ZERO: Vector2 = Vector2 { x: 0.0, y: 0.0 };

    pub const INTERNAL_CONST_2: Vector2 = Vector2 { x: 111.0, y: 222.0 };

    #[destack::partial(Vector2, distance_squared, block)]
    #[inline]
    /// Return the distance from the origin.
    pub const fn distance_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    #[inline]
    fn custom_function(&mut self) {
        // test
    }
}

impl Vector2 {
    // more custom implementation
}
"""

# The 'new' file generated from source
FILE_NEW: str = """\
#![destack::partial(vector, file)]

#[destack::generated(Vector2, struct, block)]
#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
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

    #[destack::partial(Vector2, distance_squared, block)]
    #[inline]
    /// Return the distance from the origin SQUARED.
    pub const fn distance_squared(&self) -> f32 {
        panic!("PLACEHOLDER: Vector2::distance_squared not implemented!")
    }
}

#[destack::partial(Vector2, Add:Vector2, block)]
// Arithmetic ops with another Vector2
impl Add for Vector2 {
    type Output = Vector2;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        panic!("PLACEHOLDER: Vector2::add not implemented!")
    }
}
"""

# The expected target file
FILE_PATCHED: str = """\
#![destack::partial(vector, file)]

#[destack::generated(Vector2, struct, block)]
#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
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

    pub const INTERNAL_CONST_2: Vector2 = Vector2 { x: 111.0, y: 222.0 };

    #[destack::generated(Vector2, ONE, line)]
    pub const ONE: Vector2 = Vector2 { x: 1.0, y: 1.0 };

    #[destack::generated(Vector2, X_AXIS, line)]
    pub const X_AXIS: Vector2 = Vector2 { x: 1.0, y: 0.0 };

    #[destack::generated(Vector2, Y_AXIS, line)]
    pub const Y_AXIS: Vector2 = Vector2 { x: 0.0, y: 1.0 };
    
    #[destack::partial(Vector2, distance_squared, block)]
    #[inline]
    /// Return the distance from the origin SQUARED.
    pub const fn distance_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y
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
        panic!("PLACEHOLDER: Vector2::add not implemented!")
    }
}
"""


def test_patch_rust_file():
    """
    Test that diff_rust_file produces correct PATCH operation with expected content.
    """
    old_file = parse_rust_file(
        source=FILE_OLD,
        local_path="simulation/geometry/vector.rs",
        source_path="destack.simulation.geometry.vector",
    )
    new_file = parse_rust_file(
        source=FILE_NEW,
        local_path="simulation/geometry/vector.rs",
        source_path="destack.simulation.geometry.vector",
    )
    operation = diff_rust_file(old_file, new_file)
    assert operation.type == RustFileOperationType.PATCH
    assert operation.combined_content is not None
    operation_file_clean = "\n".join(
        line.rstrip() for line in operation.combined_content.splitlines()
    ).strip()
    target_file_clean = "\n".join(line.rstrip() for line in FILE_PATCHED.splitlines()).strip()
    assert operation_file_clean == target_file_clean

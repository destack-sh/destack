//! The Dumper is a helper for ugly-printing AST nodes for debugging and inspection.
//! Unlike the pretty Printer, Dumper makes no attempt to look like source code;
//!  instead, Dumper is optimized for checking parse trees.
//!
//! Because the AST is a tree, we print any children as a tree.
//! If there is more than one group of children, we prefix a label for the group name (like `left`).
//! The target output is something like:
//! ```
//! Module: { name: 'module' }
//!  |- Block: { label: 'module' }
//!  |  |- Statement
//!  |  |  |- Expression::Binary { operator: Add }
//!  |  |  |  |- left: Literal::Scalar { value: 14 }
//!  |  |  |  |- right: Literal::Scalar { value: 2 }
//! ```

#![allow(clippy::match_like_matches_macro)]

use crate::{
    Block, Expression, Module, Node, NodeId, NodeTree, NodeTreeStore, PathId, PathPool,
    ScalarLiteral, StringPool, Type,
};
use destack_language_arena::StringId;

// The console colors.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum Color {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Black,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Color {
    /// Get the ANSI color code for this color.
    fn code(&self) -> &'static str {
        match self {
            Color::Red => "31",
            Color::Green => "32",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::White => "37",
            Color::Black => "30",
            Color::BrightRed => "91",
            Color::BrightGreen => "92",
            Color::BrightYellow => "93",
            Color::BrightBlue => "94",
            Color::BrightMagenta => "95",
            Color::BrightCyan => "96",
            Color::BrightWhite => "97",
        }
    }

    /// Apply this color to a string.
    fn apply(&self, text: &str) -> String {
        format!("\x1b[{}m{}\x1b[0m", self.code(), text)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DumperOptions {
    /// Spaces per indent.
    pub indent: usize,
    /// Maximum depth to dump. 0 = unlimited.
    pub max_depth: usize,
    /// Use colors.
    pub use_colors: bool,
    /// If true, show NodeId<T> raw index.
    pub show_id: bool,
    /// If true, print source Spans.
    pub show_span: bool,
}

impl Default for DumperOptions {
    fn default() -> Self {
        Self {
            indent: 2,
            max_depth: 0,
            use_colors: true,
            show_id: false,
            show_span: false,
        }
    }
}

/// A Dumper for dumping AST nodes.
#[derive(Debug)]
pub struct Dumper<'a> {
    /// The string pool.
    pub strings: &'a StringPool,
    /// The path pool.
    pub paths: &'a PathPool,
    /// The node tree.
    pub tree: &'a NodeTree,
    /// The dump options.
    pub options: DumperOptions,

    /// The buffer we're writing to.
    buffer: String,
    /// The current depth (see with_depth).
    depth: usize,
}

impl<'a> Dumper<'a> {
    /// Create a new Dumper.
    pub fn new(
        strings: &'a StringPool,
        paths: &'a PathPool,
        tree: &'a NodeTree,
        options: DumperOptions,
    ) -> Self {
        Self {
            strings,
            paths,
            tree,
            options,
            buffer: String::new(),
            depth: 0,
        }
    }

    /// Finish dumping and return the result.
    pub fn finish(self) -> String {
        self.buffer
    }

    /// Write a string to the buffer with a new depth context.
    pub fn with_depth(&mut self, lambda: impl FnOnce(&mut Self)) {
        self.depth += 1;
        lambda(self);
        self.depth -= 1;
    }

    /// Write the prefix for the current depth.
    #[inline]
    fn write_prefix(&mut self) {
        for _ in 0..self.depth {
            self.write_str("| ", Some(Color::Cyan));
        }
    }

    /// Write a newline to the buffer.
    #[inline]
    fn write_newline(&mut self) {
        self.write_str("\n", None);
    }

    /// Write a string to the buffer.
    #[inline]
    fn write_str(&mut self, s: &str, color: Option<Color>) {
        if self.options.use_colors {
            if let Some(color) = color {
                self.buffer.push_str(color.apply(s).as_str());
            } else {
                self.buffer.push_str(s);
            }
        } else {
            self.buffer.push_str(s);
        }
    }

    /// Write a char to the buffer.
    #[inline]
    fn write_char(&mut self, c: char, color: Option<Color>) {
        if self.options.use_colors {
            if let Some(color) = color {
                self.buffer.push_str(color.apply(&c.to_string()).as_str());
            } else {
                self.buffer.push(c);
            }
        } else {
            self.buffer.push(c);
        }
    }

    /// Write the string behind a StringId.
    #[inline]
    pub fn write_string_id(&mut self, id: StringId) {
        self.write_char('"', Some(Color::White));
        self.write_str(self.strings.get(id), Some(Color::Yellow));
        self.write_char('"', Some(Color::White));
    }

    /// Write the path behind a PathId.
    #[inline]
    pub fn write_path_id(&mut self, id: PathId) {
        let path = self.paths.get(id);
        for (i, string_id) in path.segments.iter().enumerate() {
            let string = self.strings.get(*string_id);
            self.write_str(string, Some(Color::Green));
            if i + 1 < path.segments.len() {
                self.write_str(".", Some(Color::White));
            }
        }
    }

    /// Helper for dumping a single node.
    pub fn node<'d>(&'d mut self, name: &str) -> NodeDumper<'d, 'a> {
        NodeDumper::new(self, name)
    }

    /// Dump something as a new line.
    pub fn dump<T: Dump>(&mut self, thing: &T) -> &mut Self {
        self.write_newline();
        self.write_prefix();
        thing.dump(self);
        self
    }
}

/// Helper for dumping a single node.
#[derive(Debug)]
pub struct NodeDumper<'d, 'p> {
    dumper: &'d mut Dumper<'p>,
    has_fields: bool,
}

impl<'d, 'p> NodeDumper<'d, 'p> {
    pub fn new(dumper: &'d mut Dumper<'p>, name: &str) -> Self {
        dumper.write_str(name, Some(Color::BrightBlue));
        Self {
            dumper,
            has_fields: false,
        }
    }

    /// Add a new field to the generated struct output.
    pub fn field<T: Dump>(&mut self, name: &str, value: &T) -> &mut Self {
        let prefix = if self.has_fields { ", " } else { " { " };
        self.dumper.write_str(prefix, Some(Color::White));
        self.dumper.write_str(name, Some(Color::Magenta));
        self.dumper.write_str(": ", Some(Color::White));
        value.dump(self.dumper);
        self.has_fields = true;
        self
    }

    /// Finish node and mark the struct as non-exhaustive (with a ..)
    pub fn finish_non_exhaustive(&mut self) -> &mut Self {
        if self.has_fields {
            self.dumper.write_str(", .. }", Some(Color::White));
        } else {
            self.dumper.write_str(" { .. }", Some(Color::White));
        }
        self
    }

    /// Finish node and close the struct as exhaustive.
    pub fn finish(&mut self) -> &mut Self {
        if self.has_fields {
            self.dumper.write_str(" }", Some(Color::White));
        }
        self
    }
}

pub trait Dump {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>);
}

// ----------------------------------------------------------------------------
// Blanket impls
// ----------------------------------------------------------------------------

/// Dump a NodeId<T> as the node it points to.
impl<T: Node + Clone + Dump> Dump for NodeId<T>
where
    NodeTree: NodeTreeStore<T>,
{
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        let node = dumper.tree.get(*self);
        node.dump(dumper);
    }
}

/// Dump a &str as a string.
impl Dump for &str {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(self.as_ref(), Some(Color::BrightYellow));
    }
}

/// Dump a StringId as a string.
impl Dump for StringId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_string_id(*self);
    }
}

/// Dump a PathId as a string.
impl Dump for PathId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_path_id(*self);
    }
}

/// Dump an Option<T> as a string.
impl<T: Dump> Dump for Option<T> {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Some(v) => v.dump(dumper),
            None => dumper.write_str("<None>", Some(Color::Red)),
        }
    }
}

/// Dump a Vec<T> as a slice.
impl<T: Dump> Dump for Vec<T> {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        self.as_slice().dump(dumper)
    }
}

/// Dump a bool as a string.
impl Dump for bool {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(if *self { "true" } else { "false" }, None)
    }
}

/// Dump a u8 as a string.
impl Dump for u8 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(&self.to_string(), None)
    }
}

/// Dump an i64 as a string.
impl Dump for i64 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(&self.to_string(), None)
    }
}

/// Dump an f64 as a string.
impl Dump for f64 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(&self.to_string(), None)
    }
}

/// Dump a char as a quoted character.
impl Dump for char {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_char('\'', None);
        dumper.write_str(&self.to_string(), None);
        dumper.write_char('\'', None);
    }
}

/// Dump a &[T] as a string.
impl<T: Dump> Dump for &[T] {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str("[", Some(Color::White));
        for (i, v) in self.iter().enumerate() {
            if i > 0 {
                dumper.write_str(", ", Some(Color::White));
            }
            v.dump(dumper);
        }
        dumper.write_str("]", Some(Color::White));
    }
}

// ----------------------------------------------------------------------------
// Nodes
// ----------------------------------------------------------------------------

impl Dump for Module {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Module").field("name", &self.name).finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.body);
        });
    }
}

impl Dump for Block {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Block").finish();
    }
}

impl Dump for ScalarLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ScalarLiteral::Boolean(value) => {
                let mut node_dumper = dumper.node("ScalarLiteral::Boolean");
                node_dumper.field("value", &value.to_string().as_str());
                node_dumper.finish();
            }
            ScalarLiteral::Byte(value) => {
                let mut node_dumper = dumper.node("ScalarLiteral::Byte");
                node_dumper.field("value", &value.to_string().as_str());
                node_dumper.finish();
            }
            ScalarLiteral::Integer(value, _) => {
                let mut node_dumper = dumper.node("ScalarLiteral::Integer");
                node_dumper.field("value", &value.to_string().as_str());
                node_dumper.finish();
            }
            ScalarLiteral::Float(value, _) => {
                let mut node_dumper = dumper.node("ScalarLiteral::Float");
                node_dumper.field("value", &value.to_string().as_str());
                node_dumper.finish();
            }
            ScalarLiteral::Character(value) => {
                let mut node_dumper = dumper.node("ScalarLiteral::Character");
                node_dumper.field("value", &value.to_string().as_str());
                node_dumper.finish();
            }
            _ => {
                let mut node_dumper = dumper.node("ScalarLiteral");
                node_dumper.finish_non_exhaustive();
            }
        }
    }
}

impl Dump for Expression {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            // path
            Expression::Path { path } => {
                let mut node_dumper = dumper.node("Expression::Path");
                node_dumper.field("path", path);
                node_dumper.finish();
            }
            // scalar literal
            Expression::ScalarLiteral(literal) => {
                let mut node_dumper = dumper.node("Expression::ScalarLiteral");
                node_dumper.field("value", literal);
                node_dumper.finish();
            }
            // unary
            Expression::Unary { operator, right } => {
                let mut node_dumper = dumper.node("Expression::Unary");
                let operator_str = format!("{:?}", operator.as_token_type());
                node_dumper.field("operator", &operator_str.as_str());
                node_dumper.finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(right);
                });
            }
            // binary
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let mut node_dumper = dumper.node("Expression::Binary");
                let operator_str = format!("{:?}", operator.as_token_type());
                node_dumper.field("operator", &operator_str.as_str());
                node_dumper.finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(left);
                    dumper.dump(right);
                });
            }
            // identifier
            _ => {
                let mut node_dumper = dumper.node("Expression");
                node_dumper.finish_non_exhaustive();
            }
        }
    }
}

impl Dump for Type {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Type").finish();
    }
}

// -------- Structs --------
// impl_dump_struct!(TypeName => "Header"; fields[a, b]; children[c, d]);
macro_rules! impl_dump_struct {
	// fields + children
	($ty:ty => $hdr:expr ; fields[$($f:ident),* $(,)?] ; children[$($c:ident),* $(,)?]) => {
			impl Dump for $ty {
					fn dump<'a>(&self, d: &mut Dumper<'a>) {
							let mut n = d.node($hdr);
							$( n.field(stringify!($f), &self.$f); )*
							n.finish();
							d.with_depth(|d| { $( d.dump(&self.$c); )* });
					}
			}
	};
	// fields only
	($ty:ty => $hdr:expr ; fields[$($f:ident),* $(,)?]) => {
			impl Dump for $ty {
					fn dump<'a>(&self, d: &mut Dumper<'a>) {
							let mut n = d.node($hdr);
							$( n.field(stringify!($f), &self.$f); )*
							n.finish();
					}
			}
	};
	// children only
	($ty:ty => $hdr:expr ; children[$($c:ident),* $(,)?]) => {
			impl Dump for $ty {
					fn dump<'a>(&self, d: &mut Dumper<'a>) {
							let mut n = d.node($hdr); n.finish();
							d.with_depth(|d| { $( d.dump(&self.$c); )* });
					}
			}
	};
	// nothing
	($ty:ty => $hdr:expr) => {
			impl Dump for $ty {
					fn dump<'a>(&self, d: &mut Dumper<'a>) {
							let mut n = d.node($hdr); n.finish();
					}
			}
	};
}

// -------- Enums --------

macro_rules! impl_dump_enum {
	// with fallback
	($ty:ty { $($body:tt)* _ => non_exhaustive(); }) => {
			impl_dump_enum!(@inner $ty true { $($body)* });
	};
	// without fallback
	($ty:ty { $($body:tt)* }) => {
			impl_dump_enum!(@inner $ty false { $($body)* });
	};

	// inner: iterate variants as `tt ;`
	(@inner $ty:ty $fallback:tt { $($variant:tt ;)* }) => {
			impl Dump for $ty {
					fn dump<'a>(&self, d: &mut Dumper<'a>) {
							match self {
									$( impl_dump_enum!(@emit_variant $ty $variant); )*
									impl_dump_enum!(@emit_fallback $fallback, $ty)
							}
					}
			}
	};

	// emit a struct-like variant:  Variant { binds } [header(..)] [fields[..]] [children[..]]
	(@emit_variant $ty:ty
			$v:ident { $($bind:ident),* $(,)? }
			$(header($hdr:expr))?
			$(fields[$($ff:ident),* $(,)?])?
			$(children[$($cc:ident),* $(,)?])?
	) => {
			Self::$v { $($bind),* } => {
					let header = impl_dump_enum!(@hdr stringify!($ty), stringify!($v) $(, $hdr)?);
					let mut n = d.node(header);
					$( $( n.field(stringify!($ff), $ff); )* )?
					n.finish();
					d.with_depth(|d| { $( $( d.dump($cc); )* )? });
			}
	};

	// emit a tuple-like variant:   Variant ( binds ) [header(..)] [fields[..]] [children[..]]
	(@emit_variant $ty:ty
			$v:ident ( $($bind:ident),* $(,)? )
			$(header($hdr:expr))?
			$(fields[$($ff:ident),* $(,)?])?
			$(children[$($cc:ident),* $(,)?])?
	) => {
			Self::$v( $($bind),* ) => {
					let header = impl_dump_enum!(@hdr stringify!($ty), stringify!($v) $(, $hdr)?);
					let mut n = d.node(header);
					$( $( n.field(stringify!($ff), $ff); )* )?
					n.finish();
					d.with_depth(|d| { $( $( d.dump($cc); )* )? });
			}
	};

	// header helper
	(@hdr $ty:expr, $v:expr) => { concat!($ty, "::", $v) };
	(@hdr $ty:expr, $v:expr, $hdr:expr) => { $hdr };

	// fallback selector
	(@emit_fallback true, $ty:ty) => {
			#[allow(unreachable_patterns)]
			_ => { let mut n = d.node(stringify!($ty)); n.finish_non_exhaustive(); }
	};
	(@emit_fallback false, $ty:ty) => {};
}

impl_dump_enum!(
    ScalarLiteral {
				Boolean(value): value
				Byte(value): value
				Integer(value, ty): value, ty
				Float(value, ty): value, ty
				Character(value): value
				_ => non_exhaustive();
    }
);

use serde::{Deserialize, Serialize};

use crate::{
    Block, BorrowedPath, Function, Global, Lifetime, Local, LocalNodeId, Path, Projection,
    ReferenceKind, Tree, Type, TypedValue, Value,
};

/// One value reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueReference {
    /// One concrete SSA value.
    Value(Value),
    /// One required value that was omitted.
    Missing,
    /// One malformed value fragment.
    Error,
}

impl From<Value> for ValueReference {
    fn from(value: Value) -> Self {
        Self::Value(value)
    }
}

impl From<&Value> for ValueReference {
    fn from(value: &Value) -> Self {
        Self::Value(*value)
    }
}

impl ValueReference {
    /// Return the concrete value when present.
    #[inline]
    pub fn value(self) -> Option<Value> {
        match self {
            Self::Value(value) => Some(value),
            Self::Missing | Self::Error => None,
        }
    }

    /// Replace this reference when it points at one value.
    #[inline]
    pub fn replace_value(&mut self, from: Value, to: Value) {
        if *self == Self::Value(from) {
            *self = Self::Value(to);
        }
    }
}

/// One type reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeReference {
    /// One concrete type.
    Type {
        /// The referenced type.
        ty: LocalNodeId<Type>,
        /// The lifetime arguments.
        lifetimes: Vec<Lifetime>,
    },
    /// One required type that was omitted.
    Missing,
    /// One malformed type fragment.
    Error,
}

impl From<LocalNodeId<Type>> for TypeReference {
    fn from(ty: LocalNodeId<Type>) -> Self {
        Self::new(ty, Vec::new())
    }
}

impl From<&LocalNodeId<Type>> for TypeReference {
    fn from(ty: &LocalNodeId<Type>) -> Self {
        Self::new(*ty, Vec::new())
    }
}

impl TypeReference {
    /// Create a type reference from a type and lifetime arguments.
    pub fn new(ty: LocalNodeId<Type>, lifetimes: Vec<Lifetime>) -> Self {
        Self::Type { ty, lifetimes }
    }

    /// Return the concrete type when present.
    #[inline]
    pub fn ty(&self) -> Option<LocalNodeId<Type>> {
        match self {
            Self::Type { ty, .. } => Some(*ty),
            Self::Missing | Self::Error => None,
        }
    }

    /// Return the lifetime arguments.
    #[inline]
    pub fn lifetimes(&self) -> &[Lifetime] {
        match self {
            Self::Type { lifetimes, .. } => lifetimes,
            Self::Missing | Self::Error => &[],
        }
    }

    /// Return borrowed reference-like paths carried by this type reference.
    pub fn borrowed_paths(&self, tree: &Tree) -> Vec<BorrowedPath> {
        let lifetimes = self.lifetimes().to_vec();
        let mut borrowed_paths = Vec::new();

        self.collect_borrowed_paths_into(tree, &lifetimes, Path::root(), &mut borrowed_paths);

        borrowed_paths
    }

    /// Collect borrowed paths carried by this type reference into an output vector.
    fn collect_borrowed_paths_into(
        &self,
        tree: &Tree,
        lifetimes: &[Lifetime],
        path: Path,
        borrowed_paths: &mut Vec<BorrowedPath>,
    ) {
        let lifetimes = if self.lifetimes().is_empty() {
            lifetimes.to_vec()
        } else {
            self.lifetimes()
                .iter()
                .map(|lifetime| tree.substitute_lifetime(lifetime, lifetimes))
                .collect()
        };
        let Some(ty) = self.ty() else {
            return;
        };

        Self::collect_type_borrowed_paths_into(tree, ty, &lifetimes, path, borrowed_paths);
    }

    /// Collect borrowed paths carried by one type into an output vector.
    fn collect_type_borrowed_paths_into(
        tree: &Tree,
        ty: LocalNodeId<Type>,
        lifetimes: &[Lifetime],
        path: Path,
        borrowed_paths: &mut Vec<BorrowedPath>,
    ) {
        match tree.get(ty) {
            // record borrowed reference-like leaves
            Type::Reference {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::TensorView {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            } => {
                let lifetime = tree.substitute_lifetime(lifetime, lifetimes);
                if !lifetime.is_empty() {
                    borrowed_paths.push(BorrowedPath { path, lifetime });
                }
            }
            // descend into fixed-field aggregates
            Type::Struct { fields, .. } => {
                for (index, field) in fields.iter().enumerate() {
                    let field = tree.get(*field);
                    let path = path.clone().with_projection(Projection::Field {
                        index: index as u32,
                    });

                    field
                        .ty
                        .collect_borrowed_paths_into(tree, lifetimes, path, borrowed_paths);
                }
            }
            // descend into positional aggregates
            Type::Tuple { elements, .. } => {
                for (index, element) in elements.iter().enumerate() {
                    let path = path.clone().with_projection(Projection::Field {
                        index: index as u32,
                    });

                    element.collect_borrowed_paths_into(tree, lifetimes, path, borrowed_paths);
                }
            }
            // transparent storage wrappers
            Type::Newtype { inner, .. }
            | Type::Dynamic { constraint: inner }
            | Type::Uninit { value: inner }
            | Type::Atomic { value: inner } => {
                inner.collect_borrowed_paths_into(tree, lifetimes, path, borrowed_paths);
            }
            // descend into each variant payload shape
            Type::Variant { cases, .. } => {
                for case in cases {
                    let path = path.clone().with_projection(Projection::Variant {
                        tag: case.tag.clone(),
                    });

                    case.ty
                        .collect_borrowed_paths_into(tree, lifetimes, path, borrowed_paths);
                }
            }
            // collapse indexed containers to any-element paths
            Type::Array { element, .. }
            | Type::Slice { element, .. }
            | Type::Vector { element, .. }
            | Type::Tensor { element, .. }
            | Type::TensorView { element, .. } => {
                let path = path.with_projection(Projection::AnyElement);

                element.collect_borrowed_paths_into(tree, lifetimes, path, borrowed_paths);
            }
            _ => {}
        }
    }
}

/// One block reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockReference {
    /// One concrete block.
    Block(LocalNodeId<Block>),
    /// One required block that was omitted.
    Missing,
    /// One malformed block fragment.
    Error,
}

impl From<LocalNodeId<Block>> for BlockReference {
    fn from(block: LocalNodeId<Block>) -> Self {
        Self::Block(block)
    }
}

impl From<&LocalNodeId<Block>> for BlockReference {
    fn from(block: &LocalNodeId<Block>) -> Self {
        Self::Block(*block)
    }
}

impl BlockReference {
    /// Return the concrete block when present.
    #[inline]
    pub fn block(self) -> Option<LocalNodeId<Block>> {
        match self {
            Self::Block(block) => Some(block),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One function reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionReference {
    /// One concrete function.
    Function(LocalNodeId<Function>),
    /// One required function that was omitted.
    Missing,
    /// One malformed function fragment.
    Error,
}

impl From<LocalNodeId<Function>> for FunctionReference {
    fn from(function: LocalNodeId<Function>) -> Self {
        Self::Function(function)
    }
}

impl From<&LocalNodeId<Function>> for FunctionReference {
    fn from(function: &LocalNodeId<Function>) -> Self {
        Self::Function(*function)
    }
}

impl FunctionReference {
    /// Return the concrete function when present.
    #[inline]
    pub fn function(self) -> Option<LocalNodeId<Function>> {
        match self {
            Self::Function(function) => Some(function),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One local reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocalReference {
    /// One concrete local.
    Local(LocalNodeId<Local>),
    /// One required local that was omitted.
    Missing,
    /// One malformed local fragment.
    Error,
}

impl From<LocalNodeId<Local>> for LocalReference {
    fn from(local: LocalNodeId<Local>) -> Self {
        Self::Local(local)
    }
}

impl From<&LocalNodeId<Local>> for LocalReference {
    fn from(local: &LocalNodeId<Local>) -> Self {
        Self::Local(*local)
    }
}

impl LocalReference {
    /// Return the concrete local when present.
    #[inline]
    pub fn local(self) -> Option<LocalNodeId<Local>> {
        match self {
            Self::Local(local) => Some(local),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One global reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GlobalReference {
    /// One concrete global.
    Global(LocalNodeId<Global>),
    /// One required global that was omitted.
    Missing,
    /// One malformed global fragment.
    Error,
}

impl From<LocalNodeId<Global>> for GlobalReference {
    fn from(global: LocalNodeId<Global>) -> Self {
        Self::Global(global)
    }
}

impl From<&LocalNodeId<Global>> for GlobalReference {
    fn from(global: &LocalNodeId<Global>) -> Self {
        Self::Global(*global)
    }
}

impl From<i128> for IntegerReference {
    fn from(value: i128) -> Self {
        Self::Integer(value)
    }
}

impl From<&i128> for IntegerReference {
    fn from(value: &i128) -> Self {
        Self::Integer(*value)
    }
}

impl GlobalReference {
    /// Return the concrete global when present.
    #[inline]
    pub fn global(self) -> Option<LocalNodeId<Global>> {
        match self {
            Self::Global(global) => Some(global),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One integer reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntegerReference {
    /// One concrete integer.
    Integer(i128),
    /// One required integer that was omitted.
    Missing,
    /// One malformed integer fragment.
    Error,
}

impl IntegerReference {
    /// Return the concrete integer when present.
    #[inline]
    pub fn integer(self) -> Option<i128> {
        match self {
            Self::Integer(value) => Some(value),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    /// The SSA value.
    pub value: ValueReference,
    /// The parameter type.
    pub ty: TypeReference,
}

impl Parameter {
    /// Return the concrete typed value when present.
    #[inline]
    pub fn typed_value(&self) -> Option<TypedValue> {
        let value = self.value.value()?;
        let ty = self.ty.ty()?;

        Some(TypedValue::new(value, ty))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Access, BorrowedPath, Constant, Copy, Field, Lifetime, Nullability, Path, Projection,
        ReferenceKind, Space, Tree, Type, TypeReference, VariantCase,
    };

    #[test]
    fn test_collect_borrowed_paths_for_fields_and_variants() {
        let mut tree = Tree::new();
        let int = tree.insert_type(Type::Int {
            width: 32,
            is_signed: true,
        });

        let left = tree.insert_type(Type::Reference {
            kind: ReferenceKind::Borrowed,
            lifetime: Lifetime::slot(0),
            space: Space::Local,
            access: Access::Readonly,
            pointee: int.into(),
            nullability: Nullability::None,
        });
        let right = tree.insert_type(Type::Reference {
            kind: ReferenceKind::Borrowed,
            lifetime: Lifetime::slot(1),
            space: Space::Local,
            access: Access::Readonly,
            pointee: int.into(),
            nullability: Nullability::None,
        });

        let left_field = tree.insert(Field {
            name: None,
            ty: left.into(),
        });
        let structure = tree.insert_type(Type::Struct {
            fields: vec![left_field],
            copy: Copy::Yes,
        });
        let variant = tree.insert_type(Type::Variant {
            tag: int.into(),
            storage: right.into(),
            cases: vec![VariantCase {
                tag: Constant::int32(7),
                ty: right.into(),
            }],
            copy: Copy::Yes,
        });
        let tuple = tree.insert_type(Type::Tuple {
            elements: vec![structure.into(), variant.into()],
            copy: Copy::Yes,
        });

        let ty = TypeReference::from(tuple);
        let paths = ty.borrowed_paths(&tree);

        assert_eq!(
            paths,
            vec![
                BorrowedPath {
                    path: Path::root()
                        .with_projection(Projection::Field { index: 0 })
                        .with_projection(Projection::Field { index: 0 }),
                    lifetime: Lifetime::slot(0),
                },
                BorrowedPath {
                    path: Path::root()
                        .with_projection(Projection::Field { index: 1 })
                        .with_projection(Projection::Variant {
                            tag: Constant::int32(7),
                        }),
                    lifetime: Lifetime::slot(1),
                },
            ],
        );
    }
}

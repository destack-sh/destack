use crate::{
    Access, AssignOperator, Asynchrony, BinaryOperator, BindingKeyword, BlockContext, BlockForm,
    DecoratorPosition, DependencyBinding, ExportKind, FunctionForm, FunctionPhase, FunctionRole,
    IfForm, ImportAttributeClause, InferForm, LetKind, Literal, LocalNodeId, LocalNodeIdAny,
    MappedTypeModifier, MethodAbstraction, Mutability, Name, Node, Path, PostfixPosition, RangeEnd,
    StringId, SwitchSelector, TemplateChunk, ThisForm, TupleForm, TypeLiteral, UnaryOperator,
    VarianceBound, VarianceModifier, Visibility, WhereRelation, WhileForm, YieldCardinality,
};

/// Rewrite every local node id one authored tree value embeds.
pub trait NodeFold {
    /// Rewrite every local node id this value embeds.
    fn map_nodes<E>(
        &mut self,
        map: &mut impl FnMut(LocalNodeIdAny) -> Result<u32, E>,
    ) -> Result<(), E>;
}

impl<T: Node> NodeFold for LocalNodeId<T> {
    fn map_nodes<E>(
        &mut self,
        map: &mut impl FnMut(LocalNodeIdAny) -> Result<u32, E>,
    ) -> Result<(), E> {
        let node = LocalNodeIdAny::new(self.id, T::TYPE);
        self.id = map(node)?;

        Ok(())
    }
}

impl<T: NodeFold> NodeFold for Option<T> {
    fn map_nodes<E>(
        &mut self,
        map: &mut impl FnMut(LocalNodeIdAny) -> Result<u32, E>,
    ) -> Result<(), E> {
        if let Some(value) = self {
            value.map_nodes(map)?;
        }

        Ok(())
    }
}

impl<T: NodeFold> NodeFold for Box<T> {
    fn map_nodes<E>(
        &mut self,
        map: &mut impl FnMut(LocalNodeIdAny) -> Result<u32, E>,
    ) -> Result<(), E> {
        self.as_mut().map_nodes(map)
    }
}

impl<T: NodeFold> NodeFold for Vec<T> {
    fn map_nodes<E>(
        &mut self,
        map: &mut impl FnMut(LocalNodeIdAny) -> Result<u32, E>,
    ) -> Result<(), E> {
        for value in self {
            value.map_nodes(map)?;
        }

        Ok(())
    }
}

impl<T: NodeFold, const N: usize> NodeFold for [T; N] {
    fn map_nodes<E>(
        &mut self,
        map: &mut impl FnMut(LocalNodeIdAny) -> Result<u32, E>,
    ) -> Result<(), E> {
        for value in self {
            value.map_nodes(map)?;
        }

        Ok(())
    }
}

impl<A: NodeFold, B: NodeFold> NodeFold for (A, B) {
    fn map_nodes<E>(
        &mut self,
        map: &mut impl FnMut(LocalNodeIdAny) -> Result<u32, E>,
    ) -> Result<(), E> {
        self.0.map_nodes(map)?;

        self.1.map_nodes(map)
    }
}

/// Declare authored scalar leaves that contain no local node ids.
macro_rules! node_fold_leaves {
    ($($ty:ty),* $(,)?) => {
        $(
            impl NodeFold for $ty {
                fn map_nodes<E>(
                    &mut self,
                    _map: &mut impl FnMut(LocalNodeIdAny) -> Result<u32, E>,
                ) -> Result<(), E> {
                    Ok(())
                }
            }
        )*
    };
}

node_fold_leaves!(
    bool,
    char,
    i8,
    i16,
    i32,
    i64,
    i128,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    String,
    StringId,
    AssignOperator,
    Asynchrony,
    BinaryOperator,
    BindingKeyword,
    BlockContext,
    BlockForm,
    DecoratorPosition,
    DependencyBinding,
    ExportKind,
    FunctionForm,
    FunctionPhase,
    FunctionRole,
    IfForm,
    ImportAttributeClause,
    InferForm,
    LetKind,
    Literal,
    MappedTypeModifier,
    MethodAbstraction,
    Access,
    Mutability,
    Name,
    Path,
    PostfixPosition,
    RangeEnd,
    SwitchSelector,
    TemplateChunk,
    ThisForm,
    TupleForm,
    TypeLiteral,
    UnaryOperator,
    VarianceBound,
    VarianceModifier,
    Visibility,
    WhereRelation,
    WhileForm,
    YieldCardinality,
);

use destack_dir as dir;

/// Type inference variable used while solving checked DIR.
///
/// The id is local to one check solve and is never written into DIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::check) struct TypeInferId(pub(in crate::check) u32);

/// Static inference variable used while solving checked DIR.
///
/// The id is local to one check solve and is never written into DIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::check) struct StaticInferId(pub(in crate::check) u32);

/// Current solved state for one type inference variable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::check) struct TypeSlot {
    /// The solved type id.
    pub(in crate::check) ty: Option<dir::LocalTypeId>,
}

/// Current solved state for one static inference variable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::check) struct StaticSlot {
    /// The solved static value id.
    pub(in crate::check) value: Option<dir::LocalStaticId>,
}

/// The origin of one type inference variable.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum InferOrigin {
    /// Type attached to a DIR node.
    ///
    /// ```ds
    /// let value = source;
    /// // type(source) belongs to the source expression node
    /// ```
    Node(dir::GlobalNodeIdAny),
    /// Type attached to a DIR symbol.
    ///
    /// ```ds
    /// let value: int32;
    /// // type(value) belongs to the value symbol
    /// ```
    Symbol(dir::GlobalSymbolId),
    /// Synthetic type introduced by inference.
    ///
    /// ```ds
    /// const choose = condition ? left : right;
    /// // type(choose) may use a synthetic join variable before solving
    /// ```
    Synthetic,
}

/// The origin of one static inference variable.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum StaticInferOrigin {
    /// Static term attached to a DIR node.
    ///
    /// ```ds
    /// Array<T, N + 1>
    /// // N + 1 belongs to a static expression node
    /// ```
    Node(dir::GlobalNodeIdAny),
    /// Static term attached to a DIR symbol.
    ///
    /// ```ds
    /// static const length = 4;
    /// // length owns the static value 4
    /// ```
    Symbol(dir::GlobalSymbolId),
    /// Synthetic static term introduced by inference.
    ///
    /// ```ds
    /// type Chunk<T, N> = Array<T, N * 2>;
    /// // N * 2 may use a synthetic static variable before solving
    /// ```
    Synthetic,
}

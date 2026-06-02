use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use crate::check::{
    AssignPatternTerm, AwaitTerm, CallTerm, ConstructTerm, FormTerm, FunctionTerm, IdentityTerm,
    ImportMetaTerm, IndexSetTerm, IndexTerm, InstanceCheckTerm, KeyMembershipTerm, LayoutTerm,
    MemberCallTerm, MemberTerm, OperatorTerm, PatternTerm, RangeValueTerm, ReceiverTerm, ShapeTerm,
    StaticTerm, SuperTerm, TaggedTemplateTerm, TemplateTerm, TreeTerm, TryFailureTerm, TryTerm,
    TypeOperationTerm, TypeTerm, TypeValueTerm, YieldTerm,
};

/// Typed id for one check term in a component term table.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::check) struct TermId<T: Term> {
    /// The term index inside its typed arena.
    pub(in crate::check) id: u32,
    _ty: PhantomData<fn() -> T>,
}

impl<T: Term> Clone for TermId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Term> Copy for TermId<T> {}

impl<T: Term> TermId<T> {
    /// Create a term id.
    pub(in crate::check) fn new(id: u32) -> Self {
        Self {
            id,
            _ty: PhantomData,
        }
    }

    /// Return the term index.
    pub(in crate::check) fn index(self) -> usize {
        self.id as usize
    }
}

impl<T: Term> Debug for TermId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(std::any::type_name::<T>())
            .field(&self.id)
            .finish()
    }
}

/// Term kind stored in a component term table.
pub(in crate::check) trait Term: Sized {
    /// Return this term kind's arena.
    fn arena(table: &TermTable) -> &[Self];

    /// Return this term kind's mutable arena.
    fn arena_mut(table: &mut TermTable) -> &mut Vec<Self>;
}

macro_rules! define_term_table {
    ($($field:ident: $term:ty),+ $(,)?) => {
        /// Component arena for check terms.
        #[derive(Debug, Default)]
        pub(in crate::check) struct TermTable {
            $(
                pub(in crate::check) $field: Vec<$term>,
            )+
        }

        $(
            impl Term for $term {
                fn arena(table: &TermTable) -> &[Self] {
                    &table.$field
                }

                fn arena_mut(table: &mut TermTable) -> &mut Vec<Self> {
                    &mut table.$field
                }
            }
        )+

        impl TermTable {
            /// Return the total number of stored terms.
            pub(in crate::check) fn len(&self) -> usize {
                0 $(+ self.$field.len())+
            }

            /// Append another term table into this table.
            pub(in crate::check) fn append(&mut self, mut other: Self) {
                $(
                    self.$field.append(&mut other.$field);
                )+
            }
        }
    };
}

define_term_table! {
    assign_patterns: AssignPatternTerm,
    awaits: AwaitTerm,
    calls: CallTerm,
    constructs: ConstructTerm,
    forms: FormTerm,
    functions: FunctionTerm,
    identities: IdentityTerm,
    import_metas: ImportMetaTerm,
    index_sets: IndexSetTerm,
    indexes: IndexTerm,
    instance_checks: InstanceCheckTerm,
    key_memberships: KeyMembershipTerm,
    layouts: LayoutTerm,
    member_calls: MemberCallTerm,
    members: MemberTerm,
    operators: OperatorTerm,
    patterns: PatternTerm,
    range_values: RangeValueTerm,
    receivers: ReceiverTerm,
    shapes: ShapeTerm,
    statics: StaticTerm,
    supers: SuperTerm,
    tagged_templates: TaggedTemplateTerm,
    templates: TemplateTerm,
    trees: TreeTerm,
    try_failures: TryFailureTerm,
    tries: TryTerm,
    type_operations: TypeOperationTerm,
    type_values: TypeValueTerm,
    types: TypeTerm,
    yields: YieldTerm,
}

impl TermTable {
    /// Create an empty term table.
    pub(in crate::check) fn new() -> Self {
        Self::default()
    }

    /// Push one term into its arena.
    pub(in crate::check) fn push<T: Term>(&mut self, term: T) {
        T::arena_mut(self).push(term);
    }
}

use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::check::{
    AssignPatternTerm, AwaitTerm, CallTerm, ConstructTerm, FormTerm, FunctionTerm, IdentityTerm,
    ImportMetaTerm, IndexSetTerm, IndexTerm, InstanceCheckTerm, KeyMembershipTerm, LayoutTerm,
    MemberCallTerm, MemberTerm, OperatorTerm, PatternTerm, RangeTerm, ReceiverTerm, ShapeTerm,
    StaticTerm, SuperTerm, TaggedTemplateTerm, TemplateTerm, TreeTerm, TryFailureTerm, TryTerm,
    TypeOperationTerm, TypeTerm, TypeValueTerm, YieldTerm,
};

/// Typed id for one check term in a component term table.
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

impl<T: Term> PartialEq for TermId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Term> Eq for TermId<T> {}

impl<T: Term> PartialOrd for TermId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Term> Ord for TermId<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T: Term> Hash for TermId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

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
pub(in crate::check) trait Term: Sized + PartialEq {
    /// Return this term kind's arena.
    fn arena(table: &TermTable) -> &[Self];

    /// Return this term kind's mutable arena.
    fn arena_mut(table: &mut TermTable) -> &mut Vec<Self>;

    /// Push one term and return its id.
    fn push(table: &mut TermTable, term: Self) -> TermId<Self> {
        let id = TermId::new(Self::arena(table).len() as u32);
        Self::arena_mut(table).push(term);

        id
    }

    /// Truncate this term kind to one arena length.
    fn truncate(table: &mut TermTable, len: usize) {
        Self::arena_mut(table).truncate(len);
    }
}

macro_rules! define_term_table {
    (
        linear {
            $($field:ident: $term:ty),+ $(,)?
        }
    ) => {
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

        /// Rollback cursor for component check terms.
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
        pub(in crate::check) struct TermCursor {
            $(
                pub(in crate::check) $field: usize,
            )+
        }

        impl TermCursor {
            /// Return the cursor for one term table.
            fn new(table: &TermTable) -> Self {
                Self {
                    $(
                        $field: table.$field.len(),
                    )+
                }
            }

            /// Truncate one term table to this cursor.
            fn truncate(self, table: &mut TermTable) {
                $(
                    <$term as Term>::truncate(table, self.$field);
                )+
            }
        }

        impl TermTable {
            /// Return the total number of stored terms.
            pub(in crate::check) fn len(&self) -> usize {
                0 $(+ self.$field.len())+
            }

            /// Return one rollback cursor.
            pub(in crate::check) fn cursor(&self) -> TermCursor {
                TermCursor::new(self)
            }

            /// Truncate this table to one rollback cursor.
            pub(in crate::check) fn truncate(&mut self, cursor: TermCursor) {
                cursor.truncate(self);
            }
        }
    };
}

define_term_table! {
    linear {
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
        range_values: RangeTerm,
        receivers: ReceiverTerm,
        shapes: ShapeTerm,
        statics: StaticTerm,
        supers: SuperTerm,
        tagged_templates: TaggedTemplateTerm,
        templates: TemplateTerm,
        trees: TreeTerm,
        try_failures: TryFailureTerm,
        tries: TryTerm,
        types: TypeTerm,
        type_operations: TypeOperationTerm,
        type_values: TypeValueTerm,
        yields: YieldTerm,
    }
}

impl TermTable {
    /// Create an empty term table.
    pub(in crate::check) fn new() -> Self {
        Self::default()
    }

    /// Return the id for one term.
    pub(in crate::check) fn push<T: Term>(&mut self, term: T) -> TermId<T> {
        T::push(self, term)
    }

    /// Return one term by id.
    pub(in crate::check) fn get<T: Term>(&self, id: TermId<T>) -> &T {
        &T::arena(self)[id.index()]
    }
}

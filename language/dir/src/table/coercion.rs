use destack_serde::Reflect;
use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{CastOrigin, Form, GlobalNodeIdAny, GlobalTypeId, ScalarLiteral, SegmentView, Type};

/// Cumulative checked coercions for one DIR module.
#[derive(Debug, Clone)]
pub struct CoercionTable<'a> {
    /// The module id of the coercion table.
    pub module_id: ModuleId,
    /// The ordered coercion table segments.
    segments: SegmentView<'a, CoercionSegment>,
}

impl CoercionTable<'static> {
    /// Create a coercion table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<CoercionSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a coercion table from one segment.
    pub fn from_segment(segment: Arc<CoercionSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> CoercionTable<'a> {
    /// Create a coercion table from a segment view.
    pub fn from_view(segments: SegmentView<'a, CoercionSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("coercion table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "coercion table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a coercion table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b CoercionSegment) -> CoercionTable<'b> {
        CoercionTable::from_view(self.segments.with_tail(tail))
    }

    /// Get the effective coercion for one node.
    pub fn coercion(&self, node_id: GlobalNodeIdAny) -> Option<&Coercion> {
        for segment in self.segments.iter().rev() {
            if let Some(coercion) = segment.coercion(node_id) {
                return Some(coercion);
            }
        }

        None
    }

    /// Iterate visible coercions.
    pub fn coercions(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &Coercion)> + '_ {
        self.segments
            .iter()
            .enumerate()
            .flat_map(move |(segment_index, segment)| {
                segment
                    .coercions
                    .iter()
                    .filter_map(move |(node_id, coercion)| {
                        let is_shadowed = self
                            .segments
                            .iter()
                            .skip(segment_index + 1)
                            .any(|segment| segment.coercions.contains_key(node_id));

                        (!is_shadowed).then_some((*node_id, coercion))
                    })
            })
    }

    /// Return whether this table has no coercions.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// One adjustment in an implicit coercion path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CoercionKind {
    /// Change the semantic type without changing its runtime representation.
    Direct,
    /// Borrow one value with the target lifetime and access.
    Borrow,
    /// Tag the value into or out of a union carrier.
    Union,
    /// Box the value into or out of an existential carrier, like `Dynamic<T>`.
    Existential,
    /// Convert between scalar carriers, like `int32` into `float64`.
    Scalar,
    /// Materialize one comptime scalar at its selected carrier, like `42` into `int32`.
    Widen,
    /// Change the value carrier, like `^T` into `&T` or `T[]` into `[T]`.
    Carrier,
}

impl CoercionKind {
    /// Return the stable textual name of this adjustment.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Borrow => "borrow",
            Self::Union => "union",
            Self::Existential => "existential",
            Self::Scalar => "scalar",
            Self::Widen => "widen",
            Self::Carrier => "carrier",
        }
    }
}

/// One type coercion attached to a value node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Coercion {
    /// The source type before coercion.
    pub source: GlobalTypeId,
    /// The ordered adjustments applied to the source value.
    pub adjustments: Vec<CoercionAdjustment>,
    /// How the coercion entered DIR.
    pub origin: CastOrigin,
}

/// One typed adjustment in an implicit coercion path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CoercionAdjustment {
    /// The adjustment performed.
    pub kind: CoercionKind,
    /// The type after this adjustment.
    pub target: GlobalTypeId,
    /// The source cases entering a union carrier.
    pub cases: Vec<CoercionCase>,
}

/// One source case entering a union carrier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CoercionCase {
    /// The target union case index, absent when leaving a union carrier.
    pub target: Option<u32>,
    /// The adjustments applied to the source case payload.
    pub adjustments: Vec<CoercionAdjustment>,
}

impl Coercion {
    /// Create one coercion.
    pub fn new(
        source: GlobalTypeId,
        adjustments: Vec<CoercionAdjustment>,
        origin: CastOrigin,
    ) -> Self {
        assert!(
            !adjustments.is_empty(),
            "a coercion requires at least one adjustment"
        );

        Self {
            source,
            adjustments,
            origin,
        }
    }

    /// Create one union coercion from its complete source-case map.
    pub fn union(
        source: GlobalTypeId,
        target: GlobalTypeId,
        cases: Vec<CoercionCase>,
        origin: CastOrigin,
    ) -> Self {
        let adjustment = CoercionAdjustment {
            kind: CoercionKind::Union,
            target,
            cases,
        };

        Self::new(source, vec![adjustment], origin)
    }

    /// Return the final target type.
    pub fn target(&self) -> GlobalTypeId {
        self.adjustments[self.adjustments.len() - 1].target
    }

    /// Map every type id in this coercion.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.source = map(self.source);
        for adjustment in &mut self.adjustments {
            adjustment.map_type_ids(map);
        }
    }
}

impl CoercionAdjustment {
    /// Map every type id in this adjustment.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.target = map(self.target);
        for case in &mut self.cases {
            case.map_type_ids(map);
        }
    }
}

impl CoercionCase {
    /// Map every type id in this source case.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for adjustment in &mut self.adjustments {
            adjustment.map_type_ids(map);
        }
    }
}

impl CoercionKind {
    /// Classify the adjustment between two settled, distinct type heads, or
    /// nothing when the value stores directly.
    pub fn classify(source: &Type, target: &Type) -> Option<Self> {
        // unreachable sources store nothing
        if matches!(source, Type::Never) {
            return None;
        }

        // union carriers tag their values on entry and exit
        if matches!(source, Type::Union(_)) || matches!(target, Type::Union(_)) {
            return Some(Self::Union);
        }

        // existential carriers box their values on entry and exit
        let existential = |ty: &Type| {
            matches!(
                ty,
                Type::Any | Type::Unknown | Type::Object | Type::Dynamic(_)
            )
        };
        if existential(source) || existential(target) {
            return Some(Self::Existential);
        }

        // memory forms convert when their runtime carriers differ
        if let (Type::Form(source), Type::Form(target)) = (source, target) {
            return match Self::carriers_differ(source.form, target.form) {
                true => Some(Self::Carrier),
                false => None,
            };
        }
        // placement and readonly views store as their payloads
        if let Type::Form(form) = source
            && matches!(
                form.form,
                Form::Placed { .. } | Form::Readonly | Form::Managed
            )
        {
            return None;
        }
        if let Type::Form(form) = target
            && matches!(
                form.form,
                Form::Placed { .. } | Form::Readonly | Form::Managed
            )
        {
            return None;
        }
        // owned, borrowed, and raw values convert against bare payloads
        if matches!(source, Type::Form(_)) || matches!(target, Type::Form(_)) {
            return Some(Self::Carrier);
        }

        // sized sequences and thin pointers convert into their fat carriers
        if matches!(
            (source, target),
            (Type::Array(_), Type::Slice(_))
                | (Type::FixedArray(_), Type::Slice(_))
                | (Type::FunctionPointer(_), Type::Function(_))
        ) {
            return Some(Self::Carrier);
        }

        // scalar singletons widen naturally into their base scalars
        let stores_directly = match source {
            // integer and float literals record their selected carrier
            Type::Literal(literal) => {
                if matches!(literal, ScalarLiteral::Integer(_) | ScalarLiteral::Float(_))
                    && matches!(target, Type::Primitive(_))
                    && literal.widens_to(target)
                {
                    return Some(Self::Widen);
                }

                literal.widens_to(target)
            }
            Type::Range(range) => range.widens_to(target),
            // exact keys store as their key-domain carrier
            Type::Key(key) => {
                matches!(target, Type::Primitive(primitive) if key.widens_to_primitive(*primitive))
            }
            _ => false,
        };
        if stores_directly {
            return None;
        }

        // identical scalar carriers require no runtime conversion
        if let (Type::Primitive(source), Type::Primitive(target)) = (source, target)
            && source == target
        {
            return None;
        }

        // distinct scalar carriers convert their stored values
        if let (Type::Primitive(source), Type::Primitive(target)) = (source, target)
            && source.widens_to(*target)
        {
            return Some(Self::Scalar);
        }

        None
    }

    /// Return whether two memory form constructors store different carriers.
    fn carriers_differ(source: Form, target: Form) -> bool {
        match (source, target) {
            // placement, readonly, and managed are static or transparent
            (Form::Placed { .. } | Form::Readonly | Form::Managed, _)
            | (_, Form::Placed { .. } | Form::Readonly | Form::Managed) => false,

            // static borrow parameters share the pointer carrier
            (Form::Borrowed(_), Form::Borrowed(_)) => false,

            // equal runtime carriers store directly
            (Form::Owned, Form::Owned) | (Form::Raw, Form::Raw) => false,

            // different runtime carriers convert
            _ => true,
        }
    }
}

/// Coercions added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct CoercionSegment {
    /// The module id of the coercion segment.
    pub module_id: ModuleId,
    /// Coercions keyed by the value node being coerced.
    pub(crate) coercions: IndexMap<GlobalNodeIdAny, Coercion>,
}

impl CoercionSegment {
    /// Create an empty coercion segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            coercions: IndexMap::new(),
        }
    }

    /// Bind the coercion for one value node.
    pub fn bind_coercion(
        &mut self,
        node_id: GlobalNodeIdAny,
        coercion: Coercion,
    ) -> Option<Coercion> {
        self.coercions.insert(node_id, coercion)
    }

    /// Get the coercion for one value node.
    pub fn coercion(&self, node_id: GlobalNodeIdAny) -> Option<&Coercion> {
        self.coercions.get(&node_id)
    }

    /// Iterate coercions in insertion order.
    pub fn coercions(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &Coercion)> + '_ {
        self.coercions
            .iter()
            .map(|(node_id, coercion)| (*node_id, coercion))
    }

    /// Map every type id embedded in this segment.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for coercion in self.coercions.values_mut() {
            coercion.map_type_ids(map);
        }
    }

    /// Return whether this segment has no coercions.
    pub fn is_empty(&self) -> bool {
        self.coercions.is_empty()
    }
}

use std::mem;
use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{CastOrigin, GlobalNodeIdAny, GlobalTypeId, ScalarLiteral, SegmentView, Type};

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

    /// Get the effective coercion for one value node.
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

/// One checked coercion from a source type to a target type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Coercion {
    /// The source type before coercion.
    pub source: GlobalTypeId,
    /// The ordered adjustments applied to the source value.
    pub adjustments: Vec<CoercionAdjustment>,
    /// How the coercion entered DIR.
    pub origin: CastOrigin,
}

/// One adjustment in a checked coercion path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CoercionAdjustment {
    /// Borrow one value with the target lifetime and access.
    Borrow {
        /// The borrowed type.
        target: GlobalTypeId,
    },
    /// Read one copyable value through a reference.
    Read {
        /// The value type after the read.
        target: GlobalTypeId,
    },
    /// Convert a value whose source or target is a union.
    Union {
        /// The type after this adjustment.
        target: GlobalTypeId,
        /// The selected conversion for each possible source type.
        cases: Vec<CoercionCase>,
    },
    /// Box the value into or out of an existential carrier, like `Dynamic<T>`.
    Existential {
        /// The existential type after this adjustment.
        target: GlobalTypeId,
    },
    /// Convert between scalar carriers, like `int32` into `float64`.
    Scalar {
        /// The scalar type after this adjustment.
        target: GlobalTypeId,
    },
    /// Materialize one comptime scalar at its selected carrier, like `42` into `int32`.
    Widen {
        /// The scalar type after this adjustment.
        target: GlobalTypeId,
    },
    /// Convert one tuple value into another tuple type.
    Tuple {
        /// The tuple type after this adjustment.
        target: GlobalTypeId,
    },
    /// Change the value carrier, like `^T` into `&T` or `T[]` into `[T]`.
    Carrier {
        /// The carrier type after this adjustment.
        target: GlobalTypeId,
    },
}

/// One selected conversion for a possible union source type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CoercionCase {
    /// The source type entering this case.
    pub source: GlobalTypeId,
    /// The selected target type.
    pub target: GlobalTypeId,
    /// The ordered adjustments converting the source to the target.
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

    /// Create one union coercion from its complete source mapping.
    pub fn union(
        source: GlobalTypeId,
        target: GlobalTypeId,
        cases: Vec<CoercionCase>,
        origin: CastOrigin,
    ) -> Self {
        assert!(!cases.is_empty(), "a union coercion requires source cases");

        let adjustment = CoercionAdjustment::Union { target, cases };

        Self::new(source, vec![adjustment], origin)
    }

    /// Return the final target type.
    pub fn target(&self) -> GlobalTypeId {
        self.adjustments[self.adjustments.len() - 1].target()
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
    /// Return the type after this adjustment.
    pub const fn target(&self) -> GlobalTypeId {
        match self {
            Self::Borrow { target }
            | Self::Read { target }
            | Self::Union { target, .. }
            | Self::Existential { target }
            | Self::Scalar { target }
            | Self::Widen { target }
            | Self::Tuple { target }
            | Self::Carrier { target } => *target,
        }
    }

    /// Return the stable textual name of this adjustment.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Borrow { .. } => "borrow",
            Self::Read { .. } => "read",
            Self::Union { .. } => "union",
            Self::Existential { .. } => "existential",
            Self::Scalar { .. } => "scalar",
            Self::Widen { .. } => "widen",
            Self::Tuple { .. } => "tuple",
            Self::Carrier { .. } => "carrier",
        }
    }

    /// Map every type id in this adjustment.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Borrow { target }
            | Self::Read { target }
            | Self::Existential { target }
            | Self::Scalar { target }
            | Self::Widen { target }
            | Self::Tuple { target }
            | Self::Carrier { target } => *target = map(*target),
            Self::Union { target, cases } => {
                *target = map(*target);
                for case in cases {
                    case.map_type_ids(map);
                }
            }
        }
    }

    /// Classify one adjustment between settled, unqualified type heads.
    pub fn classify(source: &Type, target: &Type, target_id: GlobalTypeId) -> Option<Self> {
        // unreachable sources store nothing
        if matches!(source, Type::Never) {
            return None;
        }

        // unions require their complete case map from the checker
        if matches!(source, Type::Union(_)) || matches!(target, Type::Union(_)) {
            return None;
        }

        // tuple conversions preserve their aggregate operation explicitly
        if matches!((source, target), (Type::Tuple(_), Type::Tuple(_))) {
            return Some(Self::Tuple { target: target_id });
        }

        // existential carriers box their values on entry and exit
        if matches!(
            source,
            Type::Any | Type::Unknown | Type::Object | Type::Dynamic(_)
        ) || matches!(
            target,
            Type::Any | Type::Unknown | Type::Object | Type::Dynamic(_)
        ) {
            return Some(Self::Existential { target: target_id });
        }

        // sized sequences and thin pointers convert into their fat carriers
        if matches!(
            (source, target),
            (Type::Array(_), Type::Slice(_))
                | (Type::FixedArray(_), Type::Slice(_))
                | (Type::FunctionPointer(_), Type::Function(_))
        ) {
            return Some(Self::Carrier { target: target_id });
        }

        // scalar singletons with distinct carriers select their conversion
        if let Type::Literal(literal) = source
            && matches!(literal, ScalarLiteral::Integer(_) | ScalarLiteral::Float(_))
            && matches!(target, Type::Primitive(_))
            && literal.widens_to(target)
        {
            return Some(Self::Widen { target: target_id });
        }

        // distinct scalar carriers convert their stored values
        if let (Type::Primitive(source), Type::Primitive(target)) = (source, target)
            && source.widens_to(*target)
        {
            return Some(Self::Scalar { target: target_id });
        }

        None
    }
}

impl CoercionCase {
    /// Map every type id in this source case.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.source = map(self.source);
        self.target = map(self.target);
        for adjustment in &mut self.adjustments {
            adjustment.map_type_ids(map);
        }
    }
}

/// Coercions added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct CoercionSegment {
    /// The module id of the coercion segment.
    pub module_id: ModuleId,
    /// Coercions keyed by their value node.
    pub(crate) coercions: IndexMap<GlobalNodeIdAny, Coercion>,
}

/// One rollback position in a coercion segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoercionMark(usize);

impl CoercionSegment {
    /// Create an empty coercion segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            coercions: IndexMap::new(),
        }
    }

    /// Mark the current segment position for later truncation.
    pub fn mark(&self) -> CoercionMark {
        CoercionMark(self.coercions.len())
    }

    /// Truncate coercions back to one mark.
    pub fn truncate_to(&mut self, mark: CoercionMark) {
        while self.coercions.len() > mark.0 {
            self.coercions.pop();
        }
    }

    /// Bind one coercion to its value node.
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
        let coercions = mem::take(&mut self.coercions);
        for (node_id, mut coercion) in coercions {
            coercion.map_type_ids(map);
            let previous = self.coercions.insert(node_id, coercion);
            assert!(
                previous.is_none(),
                "type mapping produced duplicate coercions for node {node_id:?}"
            );
        }
    }

    /// Return whether this segment has no coercions.
    pub fn is_empty(&self) -> bool {
        self.coercions.is_empty()
    }
}

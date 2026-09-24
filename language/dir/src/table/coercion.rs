use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Call, CastOrigin, GenericArgumentBinding, GlobalNodeIdAny, GlobalTypeId, InstanceKeyVisit,
    SegmentView, Type, TypeFold,
};

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
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct Coercion {
    /// The source type before coercion.
    pub source: GlobalTypeId,
    /// The ordered adjustments applied to the source value.
    pub adjustments: Vec<CoercionAdjustment>,
    /// How the coercion entered DIR.
    pub origin: CastOrigin,
}

/// One adjustment in a checked coercion path.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
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
    /// Erase the value behind its constraint representation, like an interface or `unknown`.
    Erase {
        /// The erased representation type after this adjustment.
        target: GlobalTypeId,
    },
    /// Convert between scalar representations, like `int32` into `float64`.
    Scalar {
        /// The scalar type after this adjustment.
        target: GlobalTypeId,
    },
    /// Wrap one backing value in its newtype, or unwrap it, like `number as UserId`.
    Newtype {
        /// The newtype or its backing after this adjustment.
        target: GlobalTypeId,
    },
    /// Materialize one fresh literal.
    Materialize {
        /// The representation after this adjustment.
        target: GlobalTypeId,
    },
    /// Convert one tuple value into another tuple type.
    Tuple {
        /// The tuple type after this adjustment.
        target: GlobalTypeId,
    },
    /// Transfer one owned value into managed storage, like `^T` into `T`.
    Manage {
        /// The managed type after this adjustment.
        target: GlobalTypeId,
    },
    /// Change the value representation, like `^T` into `&T` or `T[]` into `[T]`.
    Representation {
        /// The representation type after this adjustment.
        target: GlobalTypeId,
    },
    /// Own one fresh managed literal by cloning it, like `"text"` into `^string`.
    Clone {
        /// The owned type after this adjustment.
        target: GlobalTypeId,
        /// The selected clone call over the literal.
        call: Box<Call>,
    },
    /// Materialize one generic callable reference at its selected concrete instance.
    Instantiate {
        /// The concrete callable type after this adjustment.
        target: GlobalTypeId,
        /// The generic arguments selecting the instance.
        arguments: Vec<GenericArgumentBinding>,
    },
}

/// One selected conversion for a possible union source type.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
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
}

impl CoercionAdjustment {
    /// Return the type after this adjustment.
    pub const fn target(&self) -> GlobalTypeId {
        match self {
            Self::Borrow { target }
            | Self::Read { target }
            | Self::Union { target, .. }
            | Self::Erase { target }
            | Self::Scalar { target }
            | Self::Newtype { target }
            | Self::Materialize { target }
            | Self::Tuple { target }
            | Self::Clone { target, .. }
            | Self::Manage { target }
            | Self::Representation { target }
            | Self::Instantiate { target, .. } => *target,
        }
    }

    /// Return the stable textual name of this adjustment.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Borrow { .. } => "borrow",
            Self::Read { .. } => "read",
            Self::Union { .. } => "union",
            Self::Erase { .. } => "erase",
            Self::Scalar { .. } => "scalar",
            Self::Newtype { .. } => "newtype",
            Self::Materialize { .. } => "materialize",
            Self::Tuple { .. } => "tuple",
            Self::Manage { .. } => "manage",
            Self::Clone { .. } => "clone",
            Self::Representation { .. } => "representation",
            Self::Instantiate { .. } => "instantiate",
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

        // concrete objects erase into the object types they satisfy
        if matches!(source, Type::Object(shape) if !shape.declares_signatures())
            && matches!(target, Type::Object(shape) if shape.declares_signatures())
        {
            return Some(Self::Erase { target: target_id });
        }

        // sized sequences and thin pointers convert into their fat representations
        if matches!(
            (source, target),
            (Type::FixedArray(_), Type::Slice(_)) | (Type::FunctionPointer(_), Type::Function(_))
        ) {
            return Some(Self::Representation { target: target_id });
        }

        // scalar singletons are const: widening materializes them
        if let Type::Literal(literal) = source
            && matches!(target, Type::Primitive(_))
            && literal.widens_to(target)
        {
            return Some(Self::Materialize { target: target_id });
        }

        // distinct scalar representations convert their stored values
        if let (Type::Primitive(source), Type::Primitive(target)) = (source, target)
            && source != target
            && source.widens_to(*target)
        {
            return Some(Self::Scalar { target: target_id });
        }

        None
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
            coercions: IndexMap::default(),
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

    /// Return whether this segment has no coercions.
    pub fn is_empty(&self) -> bool {
        self.coercions.is_empty()
    }
}

impl TypeFold for CoercionSegment {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        for coercion in self.coercions.values_mut() {
            coercion.map_types(map)?;
        }

        Ok(())
    }
}

use serde::{Deserialize, Serialize};

use tspp_core::StringId;
use tspp_serde::Reflect;

use crate::{FieldId, GenericArgument, Symbol, Tree, Type, TypeId};

/// The dispatch shape of each dynamic constraint one module lowers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ShapeTable {
    /// The shapes sorted by constraint type.
    shapes: Vec<DynamicShape>,
}

impl ShapeTable {
    /// Record one constraint's shape, keeping the first one registered.
    pub fn insert(&mut self, shape: DynamicShape) {
        let index = self
            .shapes
            .binary_search_by_key(&shape.constraint.index(), |found| found.constraint.index());
        if let Err(index) = index {
            self.shapes.insert(index, shape);
        }
    }

    /// Return the shape one constraint dispatches through, with an application's arguments.
    pub fn shape<'a>(
        &'a self,
        tree: &'a Tree,
        constraint: TypeId,
    ) -> Option<(&'a DynamicShape, &'a [GenericArgument])> {
        // read an application through its base's shape
        let (base, arguments) = match tree.get(constraint) {
            Type::Application { base, arguments } => (*base, arguments.as_slice()),
            _ => (constraint, [].as_slice()),
        };
        let index = self
            .shapes
            .binary_search_by_key(&base.index(), |found| found.constraint.index())
            .ok()?;

        Some((&self.shapes[index], arguments))
    }

    /// Iterate the shapes in constraint order.
    pub fn iter(&self) -> impl Iterator<Item = &DynamicShape> {
        self.shapes.iter()
    }
}

/// Table shape for one dynamic constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicShape {
    /// The dynamic constraint type owning this shape.
    pub constraint: TypeId,
    /// Slots in declaration order.
    pub slots: Vec<DynamicSlot>,
    /// How a dynamic value of the constraint finds its members.
    pub lookup: ShapeLookup,
}

/// How a dynamic value finds the members of its constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ShapeLookup {
    /// By slot alone.
    Slot,
    /// By slot, and by field name for an index signature.
    Name,
}

/// Slot descriptor for a dynamic shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DynamicSlot {
    /// Field slot.
    Field {
        /// The field the slot reads.
        field: FieldId,
        /// The field name.
        name: StringId,
    },
    /// Function slot.
    Function {
        /// The requirement's name.
        name: StringId,
        /// The requirement the slot answers.
        requirement: Symbol,
        /// The function signature.
        signature: TypeId,
    },
}

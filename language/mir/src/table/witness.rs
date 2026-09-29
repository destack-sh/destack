use serde::{Deserialize, Serialize};
use tspp_core::StringId;
use tspp_serde::Reflect;

use crate::{
    DispatchSlot, FunctionId, GenericArgument, Global, LocalNodeId, StaticId, Tree, TypeId,
    erase_lifetimes,
};

/// The witness each closed type records for each interface it implements.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WitnessTable {
    /// The witnesses in record order.
    witnesses: Vec<Witness>,
}

/// One closed type's implementation of one interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Witness {
    /// The lifetime-erased closed type.
    pub concrete: TypeId,
    /// The applied interface.
    pub constraint: TypeId,
    /// The function implementing each member function.
    pub functions: Vec<WitnessFunction>,
    /// The type implementing each associated type.
    pub types: Vec<WitnessType>,
    /// The global implementing each associated const.
    pub constants: Vec<WitnessConst>,
}

/// One requirement implemented by one function or virtual table slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WitnessFunction {
    /// The requirement function.
    pub requirement: FunctionId,
    /// The implementation.
    pub implementation: WitnessImplementation,
}

/// The implementation of one witness requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum WitnessImplementation {
    /// A function or template.
    Function {
        /// The implementing function.
        function: FunctionId,
        /// The implementer's arguments with holes for the requirement's places.
        arguments: Vec<Option<GenericArgument>>,
    },
    /// A slot in the receiver class's virtual table.
    Virtual {
        /// The virtual table slot.
        slot: DispatchSlot,
    },
    /// The requirement's default body.
    Default,
    /// A slot in the dynamic receiver's own table.
    Dynamic {
        /// The dynamic slot.
        slot: DispatchSlot,
    },
}

impl WitnessImplementation {
    /// Fill each hole of one implementer's arguments in order.
    pub fn fill(
        arguments: &[Option<GenericArgument>],
        fillers: &[GenericArgument],
    ) -> Option<Vec<GenericArgument>> {
        let mut fillers = fillers.iter();
        let mut filled = Vec::with_capacity(arguments.len());
        for argument in arguments {
            filled.push(match argument {
                Some(argument) => argument.clone(),
                None => fillers.next()?.clone(),
            });
        }
        fillers.next().is_none().then_some(filled)
    }
}

/// One associated const implemented by one global.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WitnessConst {
    /// The associated const name.
    pub member: StringId,
    /// The global holding the value.
    pub global: LocalNodeId<Global>,
    /// The compile-time value, when the const has one.
    pub value: Option<StaticId>,
}

/// One associated type implemented by one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WitnessType {
    /// The associated type name.
    pub member: StringId,
    /// The implementing type.
    pub ty: TypeId,
}

impl WitnessTable {
    /// Insert one witness, replacing the one recorded for its concrete type and constraint.
    pub fn insert(&mut self, witness: Witness) {
        // replace the witness already recorded for that pair, or append a new one
        let index = self.witnesses.iter().position(|candidate| {
            candidate.concrete == witness.concrete && candidate.constraint == witness.constraint
        });
        match index {
            Some(index) => self.witnesses[index] = witness,
            None => self.witnesses.push(witness),
        }
    }

    /// Iterate every recorded witness.
    pub fn iter(&self) -> impl Iterator<Item = &Witness> {
        self.witnesses.iter()
    }

    /// Return the type one witness implements an associated type with.
    pub fn associated_type(
        &self,
        tree: &Tree,
        receiver: TypeId,
        interface: TypeId,
        member: StringId,
    ) -> Option<TypeId> {
        let concrete = erase_lifetimes(tree, receiver);
        let constraint = erase_lifetimes(tree, interface);
        let witness = self.get(concrete, constraint)?;

        witness
            .types
            .iter()
            .find(|found| found.member == member)
            .map(|found| found.ty)
    }

    /// Return the compile-time value one witness implements an associated const with.
    pub fn associated_constant(
        &self,
        tree: &Tree,
        receiver: TypeId,
        interface: TypeId,
        member: StringId,
    ) -> Option<StaticId> {
        let concrete = erase_lifetimes(tree, receiver);
        let constraint = erase_lifetimes(tree, interface);
        let witness = self.get(concrete, constraint)?;

        witness
            .constants
            .iter()
            .find(|found| found.member == member)
            .and_then(|found| found.value)
    }

    /// Return the witness recording how one type implements one interface.
    pub fn get(&self, concrete: TypeId, constraint: TypeId) -> Option<&Witness> {
        self.witnesses
            .iter()
            .find(|witness| witness.concrete == concrete && witness.constraint == constraint)
    }
}

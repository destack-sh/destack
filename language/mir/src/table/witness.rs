use destack_core::StringId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{FunctionId, Global, LocalNodeId, Tree, TypeId, erase_lifetimes};

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

/// One requirement implemented by one function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WitnessFunction {
    /// The requirement function.
    pub requirement: FunctionId,
    /// The implementing function.
    pub function: FunctionId,
}

/// One associated const implemented by one global.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WitnessConst {
    /// The associated const name.
    pub member: StringId,
    /// The global holding the value.
    pub global: LocalNodeId<Global>,
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
    pub fn insert(&mut self, tree: &mut Tree, mut witness: Witness) {
        // erase the regions off the witness key
        witness.concrete = erase_lifetimes(tree, witness.concrete);
        witness.constraint = erase_lifetimes(tree, witness.constraint);

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

    /// Return the witness recording how one type implements one interface.
    pub fn get(&self, tree: &mut Tree, concrete: TypeId, constraint: TypeId) -> Option<&Witness> {
        let concrete = erase_lifetimes(tree, concrete);
        let constraint = erase_lifetimes(tree, constraint);

        self.witnesses
            .iter()
            .find(|witness| witness.concrete == concrete && witness.constraint == constraint)
    }
}

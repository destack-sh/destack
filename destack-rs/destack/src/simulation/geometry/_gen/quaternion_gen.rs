//! destack.simulation.geometry.quaternion

#![destack::generated(destack.simulation.geometry.quaternion, file)]

use crate::Quaternion;

#[destack::generated(Quaternion, Debug, block)]
impl std::fmt::Debug for Quaternion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Quaternion")
    }
}

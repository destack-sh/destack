use destack_mir as mir;

use crate::program::{
    Instruction, Opcode, VectorCompare, VectorConvert, VectorExtract, VectorInsert, VectorReduce,
    VectorSelect, VectorShuffle, VectorSplat,
};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one vector splat.
    pub(super) fn lower_vector_splat(
        &self,
        destination: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector splat destination".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector splat value".to_string(),
        })?;

        Ok(Instruction::new(
            Opcode::VectorSplat,
            VectorSplat {
                dest: destination,
                value,
            },
        ))
    }

    /// Lower one vector extract.
    pub(super) fn lower_vector_extract(
        &self,
        destination: mir::ValueReference,
        vector: mir::ValueReference,
        index: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector extract destination".to_string(),
            })?;
        let vector = vector.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector extract input".to_string(),
        })?;
        let index = index.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector extract index".to_string(),
        })?;

        Ok(Instruction::new(
            Opcode::VectorExtract,
            VectorExtract {
                dest: destination,
                vector,
                index,
            },
        ))
    }

    /// Lower one vector insert.
    pub(super) fn lower_vector_insert(
        &self,
        destination: mir::ValueReference,
        vector: mir::ValueReference,
        index: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector insert destination".to_string(),
            })?;
        let vector = vector.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector insert input".to_string(),
        })?;
        let index = index.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector insert index".to_string(),
        })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector insert value".to_string(),
        })?;

        Ok(Instruction::new(
            Opcode::VectorInsert,
            VectorInsert {
                dest: destination,
                vector,
                index,
                value,
            },
        ))
    }

    /// Lower one vector shuffle.
    pub(super) fn lower_vector_shuffle(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        left: mir::ValueReference,
        right: mir::ValueReference,
        mask: &[u32],
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector shuffle destination".to_string(),
            })?;
        let left = left.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector shuffle left".to_string(),
        })?;
        let right = right.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector shuffle right".to_string(),
        })?;

        Ok(Instruction::new(
            Opcode::VectorShuffle,
            VectorShuffle {
                dest: destination,
                left,
                right,
                mask: pool.u32_range(mask),
            },
        ))
    }

    /// Lower one vector select.
    pub(super) fn lower_vector_select(
        &self,
        destination: mir::ValueReference,
        mask: mir::ValueReference,
        then_value: mir::ValueReference,
        else_value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector select destination".to_string(),
            })?;
        let mask = mask.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector select mask".to_string(),
        })?;
        let then_value = then_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector select then value".to_string(),
            })?;
        let else_value = else_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector select else value".to_string(),
            })?;

        Ok(Instruction::new(
            Opcode::VectorSelect,
            VectorSelect {
                dest: destination,
                mask,
                then_value,
                else_value,
            },
        ))
    }

    /// Lower one vector reduction.
    pub(super) fn lower_vector_reduce(
        &self,
        destination: mir::ValueReference,
        operator: mir::VectorReduceOperator,
        vector: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector reduce destination".to_string(),
            })?;
        let vector = vector.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector reduce input".to_string(),
        })?;

        Ok(Instruction::new(
            Opcode::VectorReduce,
            VectorReduce {
                dest: destination,
                operator,
                vector,
            },
        ))
    }

    /// Lower one vector comparison.
    pub(super) fn lower_vector_compare(
        &self,
        destination: mir::ValueReference,
        operator: mir::BinaryOperator,
        left: mir::ValueReference,
        right: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector compare destination".to_string(),
            })?;
        let left = left.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector compare left".to_string(),
        })?;
        let right = right.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector compare right".to_string(),
        })?;

        Ok(Instruction::new(
            Opcode::VectorCompare,
            VectorCompare {
                dest: destination,
                operator,
                left,
                right,
            },
        ))
    }

    /// Lower one vector conversion.
    pub(super) fn lower_vector_convert(
        &self,
        destination: mir::ValueReference,
        mode: mir::VectorConvertMode,
        vector: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector convert destination".to_string(),
            })?;
        let vector = vector.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector convert input".to_string(),
        })?;
        let dest_type = self.value_type_for_value(destination)?;
        let source_type = self.value_type_for_value(vector)?;

        Ok(Instruction::new(
            Opcode::VectorConvert,
            VectorConvert {
                dest: destination,
                mode,
                vector,
                source_type,
                dest_type,
            },
        ))
    }
}

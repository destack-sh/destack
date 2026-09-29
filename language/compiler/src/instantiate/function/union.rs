use tspp_mir as mir;

use crate::instantiate::function::Specialization;
use crate::{CompilerError, CompilerResult};

/// How one specialization reshapes a template union.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CaseMap {
    /// Every case stays at its index.
    Kept,
    /// Identical cases merged, by the new index of each case.
    Merged(Vec<u32>),
    /// Every case merged into one.
    Collapsed,
}

impl Specialization<'_, '_> {
    /// Return how the specialization reshapes one template union.
    pub(super) fn case_map(&mut self, template: mir::TypeId) -> CaseMap {
        // keep nominal variant cases
        let head = match self.source.get(template) {
            mir::Type::Application { base, .. } => *base,
            _ => template,
        };
        if matches!(self.source.get(head), mir::Type::Declaration { .. }) {
            return CaseMap::Kept;
        }

        let storage = template.storage(self.source);
        let mir::Type::Variant { cases, .. } = self.source.type_definition(storage).clone() else {
            return CaseMap::Kept;
        };

        // index each closed payload
        let payloads = cases
            .iter()
            .map(|case| self.ty(case.ty))
            .collect::<Vec<_>>();
        let (_, indices) = mir::Type::union_cases(&payloads);
        let distinct = indices.iter().max().map_or(0, |index| *index as usize + 1);

        // classify by the distinct case count
        if distinct == cases.len() {
            CaseMap::Kept
        } else if distinct == 1 {
            CaseMap::Collapsed
        } else {
            CaseMap::Merged(indices)
        }
    }

    /// Rewrite one copied instruction at its instance's union shapes.
    pub(super) fn reshape_instruction(
        &mut self,
        template: mir::LocalNodeId<mir::Instruction>,
        instruction: mir::Instruction,
    ) -> CompilerResult<mir::Instruction> {
        let template = self.source.get(template).clone();
        let mut instruction = self.reshape_variant_operation(&template, instruction)?;

        // rewrite the place of a kept operation
        if std::mem::discriminant(&template) == std::mem::discriminant(&instruction)
            && let (Some(template_place), Some(place)) = (template.place(), instruction.place_mut())
        {
            self.reshape_place(template_place, place)?;
        }

        Ok(instruction)
    }

    /// Rewrite one variant operation at its instance's union shape.
    fn reshape_variant_operation(
        &mut self,
        template: &mir::Instruction,
        instruction: mir::Instruction,
    ) -> CompilerResult<mir::Instruction> {
        match (template, instruction) {
            // construct the merged case or the collapsed payload
            (
                mir::Instruction::VariantNew { result_type, .. },
                mir::Instruction::VariantNew {
                    destination,
                    case,
                    payload,
                    result_type: closed,
                },
            ) => match self.case_map(*result_type) {
                CaseMap::Kept => Ok(mir::Instruction::VariantNew {
                    destination,
                    case,
                    payload,
                    result_type: closed,
                }),
                CaseMap::Merged(indices) => Ok(mir::Instruction::VariantNew {
                    destination,
                    case: indices[case as usize],
                    payload,
                    result_type: closed,
                }),
                CaseMap::Collapsed => match payload {
                    Some(value) => Ok(mir::Instruction::Copy { destination, value }),
                    None => Ok(mir::Instruction::Const {
                        destination,
                        value: mir::Constant::Zeroed,
                    }),
                },
            },

            // read the merged case or the collapsed value
            (
                mir::Instruction::VariantPayload { variant, .. },
                mir::Instruction::VariantPayload {
                    destination,
                    variant: closed,
                    case,
                },
            ) => {
                let ty = self.template_value_type(*variant)?;
                match self.case_map(ty) {
                    CaseMap::Kept => Ok(mir::Instruction::VariantPayload {
                        destination,
                        variant: closed,
                        case,
                    }),
                    CaseMap::Merged(indices) => Ok(mir::Instruction::VariantPayload {
                        destination,
                        variant: closed,
                        case: indices[case as usize],
                    }),
                    CaseMap::Collapsed => Ok(mir::Instruction::Copy {
                        destination,
                        value: closed,
                    }),
                }
            }

            // reject a raw tag read
            (mir::Instruction::VariantTag { variant, .. }, instruction) => {
                let ty = self.template_value_type(*variant)?;
                match self.case_map(ty) {
                    CaseMap::Kept => Ok(instruction),
                    CaseMap::Merged(_) | CaseMap::Collapsed => {
                        Err(self.unreshapable("a tag read of a union its instance merges"))
                    }
                }
            }

            (_, instruction) => Ok(instruction),
        }
    }

    /// Rewrite one variant switch at its instance's union shape.
    pub(super) fn reshape_variant_switch(
        &mut self,
        value: mir::Value,
        terminator: mir::Terminator,
    ) -> CompilerResult<mir::Terminator> {
        let ty = self.template_value_type(value)?;
        let mir::Terminator::VariantSwitch {
            value: closed,
            default,
            cases,
        } = terminator
        else {
            return Ok(terminator);
        };

        // order the cases by template index
        let mut listed = self.state.tree.get_switch_cases(cases).to_vec();
        listed.sort_by_key(|case| case.value);
        match self.case_map(ty) {
            CaseMap::Kept => Ok(mir::Terminator::VariantSwitch {
                value: closed,
                default,
                cases,
            }),
            CaseMap::Merged(indices) => {
                let mut merged = Vec::<mir::SwitchCase>::with_capacity(listed.len());
                for case in listed {
                    let value = i128::from(indices[case.value as usize]);
                    if merged.iter().all(|existing| existing.value != value) {
                        merged.push(mir::SwitchCase { value, ..case });
                    }
                }

                Ok(mir::Terminator::VariantSwitch {
                    value: closed,
                    default,
                    cases: self.state.tree.add_switch_cases(&merged),
                })
            }
            CaseMap::Collapsed => {
                let target = match listed.into_iter().next() {
                    Some(case) => case.target,
                    None => default.ok_or_else(|| CompilerError::Internal {
                        message: "a collapsed variant switch without a target".to_string(),
                    })?,
                };

                Ok(mir::Terminator::Jump { target })
            }
        }
    }

    /// Build the error for an operation a merged union cannot carry.
    fn unreshapable(&self, operation: &str) -> CompilerError {
        CompilerError::Internal {
            message: format!("{operation} cannot follow its instance's union shape"),
        }
    }
}

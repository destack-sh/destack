use destack_dir::{LocalNodeIdAny, LocalTypeId, ScalarLiteral, Type, TypeLiteral, TypeTable};

use crate::Compiler;

const MAX_RANGE_LITERAL_COUNT: i64 = 256;

impl Compiler {
    /// Check whether a scalar literal falls within a range.
    pub(crate) fn scalar_literal_in_range(
        &self,
        literal: &ScalarLiteral,
        start: &ScalarLiteral,
        end: &ScalarLiteral,
        is_inclusive: bool,
    ) -> bool {
        match (literal, start, end) {
            (
                ScalarLiteral::Integer(value),
                ScalarLiteral::Integer(start),
                ScalarLiteral::Integer(end),
            ) => {
                let end_value = if is_inclusive { *end } else { end - 1 };
                *value >= *start && *value <= end_value
            }
            (
                ScalarLiteral::Bigint(value),
                ScalarLiteral::Bigint(start),
                ScalarLiteral::Bigint(end),
            ) => {
                let end_value = if is_inclusive { *end } else { end - 1 };
                *value >= *start && *value <= end_value
            }
            (
                ScalarLiteral::Character(value),
                ScalarLiteral::Character(start),
                ScalarLiteral::Character(end),
            ) => {
                let value = *value as u32;
                let start = *start as u32;
                let end = *end as u32;
                let end_value = if is_inclusive {
                    end
                } else if end == 0 {
                    return false;
                } else {
                    end - 1
                };
                value >= start && value <= end_value
            }
            _ => false,
        }
    }

    /// Build a type id for a union of scalar literal values.
    pub(crate) fn type_id_for_scalar_literals(
        &self,
        pattern_id: LocalNodeIdAny,
        literals: Vec<ScalarLiteral>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // build literal type ids for each covered value
        let mut elements = Vec::with_capacity(literals.len());
        for literal in literals {
            let literal_id = types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(literal),
                },
                pattern_id,
            );
            elements.push(literal_id);
        }

        // collapse single literal unions
        if elements.len() == 1 {
            return elements[0];
        }

        // synthesize the union when multiple literals remain
        types.insert_type_from_any(Type::Union { elements }, pattern_id)
    }

    /// Build a union type for a scalar literal range.
    pub(crate) fn pattern_range_target_type(
        &self,
        pattern_id: LocalNodeIdAny,
        start_literal: &ScalarLiteral,
        end_literal: &ScalarLiteral,
        is_inclusive: bool,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // range guards only narrow integer, bigint, and character literals
        match (start_literal, end_literal) {
            (ScalarLiteral::Integer(start), ScalarLiteral::Integer(end)) => {
                // normalize bounds
                let end_value = if is_inclusive { *end } else { end - 1 };
                if end_value < *start {
                    return None;
                }

                // guard against large literal unions
                let count = end_value - *start + 1;
                if count > MAX_RANGE_LITERAL_COUNT {
                    return None;
                }

                // emit literal union elements
                let mut elements = Vec::with_capacity(count as usize);
                for value in *start..=end_value {
                    let literal = ScalarLiteral::Integer(value);
                    elements.push(types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(literal),
                        },
                        pattern_id,
                    ));
                }
                if elements.len() == 1 {
                    return Some(elements[0]);
                }
                // use a union for multi literal ranges
                Some(types.insert_type_from_any(Type::Union { elements }, pattern_id))
            }
            (ScalarLiteral::Bigint(start), ScalarLiteral::Bigint(end)) => {
                // normalize bounds
                let end_value = if is_inclusive { *end } else { end - 1 };
                if end_value < *start {
                    return None;
                }

                // guard against large literal unions
                let count = end_value - *start + 1;
                if count > MAX_RANGE_LITERAL_COUNT {
                    return None;
                }

                // emit literal union elements
                let mut elements = Vec::with_capacity(count as usize);
                for value in *start..=end_value {
                    let literal = ScalarLiteral::Bigint(value);
                    elements.push(types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(literal),
                        },
                        pattern_id,
                    ));
                }
                if elements.len() == 1 {
                    return Some(elements[0]);
                }
                // use a union for multi literal ranges
                Some(types.insert_type_from_any(Type::Union { elements }, pattern_id))
            }
            (ScalarLiteral::Character(start), ScalarLiteral::Character(end)) => {
                // normalize bounds
                let start_value = *start as u32;
                let end_value = *end as u32;
                let end_value = if is_inclusive {
                    end_value
                } else if end_value == 0 {
                    return None;
                } else {
                    end_value - 1
                };
                if end_value < start_value {
                    return None;
                }

                // guard against large literal unions
                let count = (end_value - start_value) as i64 + 1;
                if count > MAX_RANGE_LITERAL_COUNT {
                    return None;
                }

                // emit literal union elements
                let mut elements = Vec::with_capacity(count as usize);
                for value in start_value..=end_value {
                    let Some(character) = char::from_u32(value) else {
                        return None;
                    };
                    let literal = ScalarLiteral::Character(character);
                    elements.push(types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(literal),
                        },
                        pattern_id,
                    ));
                }
                if elements.len() == 1 {
                    return Some(elements[0]);
                }
                // use a union for multi literal ranges
                Some(types.insert_type_from_any(Type::Union { elements }, pattern_id))
            }
            _ => None,
        }
    }
}

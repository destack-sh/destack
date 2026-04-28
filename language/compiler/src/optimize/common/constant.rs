use std::collections::HashMap;

use destack_mir as mir;
use destack_mir::{
    BinaryOperator, CastOperator, Constant, Intrinsic, LocalNodeId, Tree, Type, UnaryOperator,
};

use super::{
    instruction_substitute_uses_in_tree, remap_instruction_memory_accesses,
    terminator_substitute_uses,
};

/// Constant type information for literal values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantType {
    /// Null reference constant type.
    Null,
    /// Boolean constant type.
    Boolean,
    /// Integer constant type.
    Int { width: u16, signed: bool },
    /// Floating point constant type.
    Float { width: u8 },
    /// Character constant type.
    Char,
}

/// Lookup interface for constant maps.
pub trait ConstantLookup {
    /// Return the constant value for a MIR value when known.
    fn get_constant(&self, value: mir::Value) -> Option<&mir::Constant>;
}

impl ConstantLookup for HashMap<mir::Value, mir::Constant> {
    /// Return the constant value for a MIR value when known.
    fn get_constant(&self, value: mir::Value) -> Option<&mir::Constant> {
        self.get(&value)
    }
}

/// Return the constant type for a MIR constant.
pub fn constant_type_of(constant: &Constant) -> ConstantType {
    match constant {
        Constant::Null => ConstantType::Null,
        Constant::Boolean { .. } => ConstantType::Boolean,
        Constant::Int {
            width, is_signed, ..
        } => ConstantType::Int {
            width: *width,
            signed: *is_signed,
        },
        Constant::UInt { width, .. } => ConstantType::Int {
            width: *width,
            signed: false,
        },
        Constant::Float { width, .. } => ConstantType::Float { width: *width },
        Constant::Char { .. } => ConstantType::Char,
    }
}

/// Check whether a constant type matches a MIR type.
pub fn constant_matches_type(
    constant_type: ConstantType,
    destination_type: impl Into<mir::TypeReference>,
    pointer_width_bits: u16,
    tree: &Tree,
) -> bool {
    let Some(destination_type) = destination_type.into().ty() else {
        return false;
    };

    match (constant_type, tree.get(destination_type)) {
        (ConstantType::Null, Type::Reference { is_nullable, .. })
        | (ConstantType::Null, Type::TensorView { is_nullable, .. }) => *is_nullable,
        (ConstantType::Boolean, Type::Boolean) => true,
        (ConstantType::Int { width, signed }, ty) => {
            let Some((ty_width, ty_signed)) = ty.int_info_with_pointer_width(pointer_width_bits)
            else {
                return false;
            };
            width == ty_width && signed == ty_signed
        }
        (ConstantType::Float { width }, Type::Float { width: ty_width }) => {
            u16::from(width) == *ty_width
        }
        (
            ConstantType::Char,
            Type::Int {
                width,
                is_signed: signed,
            },
        ) => *width == 32 && !*signed,
        _ => false,
    }
}

/// Extract a constant value from the given operand.
pub fn constant_for_value(
    value: mir::Value,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::Tree,
) -> Option<mir::Constant> {
    // find the instruction that defines the value
    let inst_id = *definitions.get(&value)?;
    let inst = tree.get(inst_id);

    // extract constants from direct constant instructions
    match inst {
        mir::Instruction::Const { value, .. } => Some(value.clone()),
        _ => None,
    }
}

/// Resolve constant arguments for a parameter list.
pub fn constant_arguments_for_parameters(
    arguments: &[mir::ValueReference],
    parameters: &[mir::Parameter],
    constants: &impl ConstantLookup,
    pointer_width_bits: u16,
    tree: &mir::Tree,
) -> Option<Vec<Option<mir::Constant>>> {
    // ensure argument and parameter counts match
    if arguments.len() != parameters.len() {
        return None;
    }

    // collect constant arguments in order
    let mut resolved = Vec::with_capacity(arguments.len());
    for (argument, parameter) in arguments.iter().zip(parameters.iter()) {
        let argument = argument.value()?;
        let constant = constants.get_constant(argument).and_then(|constant| {
            let constant_type = constant_type_of(constant);
            if constant_matches_type(constant_type, parameter.ty, pointer_width_bits, tree) {
                Some(constant.clone())
            } else {
                None
            }
        });
        resolved.push(constant);
    }

    Some(resolved)
}

/// Apply constant parameters inside a function body.
pub fn apply_constant_parameters(
    function_id: mir::LocalNodeId<mir::Function>,
    constants: &[Option<mir::Constant>],
    tree: &mut mir::Tree,
) -> bool {
    // prepare the substitution map and new instructions
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut new_instructions = Vec::new();

    // allocate new constants at the entry block
    let mut function = tree.get(function_id).clone();
    function.recompute_next_value_id(tree);

    let entry_id = function.entry.expect("defined function has entry block");
    let parameters = function.parameters.clone();

    for (param, constant) in parameters.iter().zip(constants.iter()) {
        // skip non constant parameters
        let Some(constant) = constant else {
            continue;
        };

        // allocate a new constant value
        let Some(parameter) = param.typed_value() else {
            continue;
        };

        let destination = function.next_typed_value(parameter.ty);
        let parameter_value = parameter.value;
        substitutions.insert(parameter_value, destination);
        new_instructions.push((destination, constant.clone()));
    }

    // write back the updated value counter
    if !new_instructions.is_empty() {
        *tree.get_mut(function_id) = function.clone();
    }

    // insert constant instructions before the entry block body
    if !new_instructions.is_empty() {
        let mut new_instruction_ids = Vec::new();

        for (destination, constant) in &new_instructions {
            let instruction_id = tree.insert(mir::Instruction::Const {
                destination: (*destination).into(),
                value: constant.clone(),
            });
            new_instruction_ids.push(instruction_id);
        }

        // insert constants at the entry block
        let entry = tree.get_mut(entry_id);
        entry
            .instructions
            .splice(0..0, new_instruction_ids.iter().copied());
    }

    // stop if no substitutions were created
    if substitutions.is_empty() {
        return false;
    }

    // substitute uses across all blocks
    let function = tree.get(function_id).clone();
    for block_id in function.blocks {
        let instruction_ids = tree.get(block_id).instructions.clone();

        // rewrite instruction operands
        for instruction_id in instruction_ids {
            let instruction = tree.get(instruction_id).clone();
            let updated = instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);
            if instruction != updated {
                *tree.get_mut(instruction_id) = updated;
                remap_instruction_memory_accesses(tree, instruction_id, &substitutions);
            }
        }

        // rewrite terminator operands
        let terminator_id = tree.get(block_id).terminator;
        let terminator = tree.get(terminator_id).clone();
        let updated = terminator_substitute_uses(&terminator, &substitutions);
        if terminator != updated {
            tree.replace(terminator_id, updated);
        }
    }

    true
}

/// Check if a constant is zero.
pub fn constant_is_zero(constant: Option<&Constant>) -> bool {
    matches!(
        constant,
        Some(Constant::Int { value: 0, .. })
            | Some(Constant::UInt { value: 0, .. })
            | Some(Constant::Float { bits: 0, .. })
            | Some(Constant::Boolean { value: false })
    )
}

/// Check if a constant is one.
pub fn constant_is_one(constant: Option<&Constant>) -> bool {
    matches!(
        constant,
        Some(Constant::Int { value: 1, .. })
            | Some(Constant::UInt { value: 1, .. })
            | Some(Constant::Boolean { value: true })
    )
}

/// Check if a constant has all bits set (i.e., -1 for signed, max for unsigned).
pub fn constant_is_all_ones(constant: Option<&Constant>) -> bool {
    match constant {
        Some(Constant::Int { value: -1, .. }) => true,
        Some(Constant::UInt { value, width }) => {
            let mask = mask_to_width(u128::MAX, *width);
            *value == mask
        }
        Some(Constant::Boolean { value: true }) => true,
        _ => false,
    }
}

/// Check if a float constant is positive zero.
pub fn constant_is_float_zero(constant: Option<&Constant>) -> bool {
    matches!(constant, Some(Constant::Float { bits: 0, .. }))
}

/// Check if a float constant is one.
pub fn constant_is_float_one(constant: Option<&Constant>) -> bool {
    match constant {
        Some(Constant::Float { bits, width: 32 }) => f32::from_bits(*bits as u32) == 1.0,
        Some(Constant::Float { bits, width: 64 }) => f64::from_bits(*bits) == 1.0,
        _ => false,
    }
}

/// Create a zero constant matching the given constant's type.
pub fn constant_zero_like(template: &Constant) -> Constant {
    match template {
        Constant::Int {
            width, is_signed, ..
        } => Constant::Int {
            value: 0,
            width: *width,
            is_signed: *is_signed,
        },
        Constant::UInt { width, .. } => Constant::UInt {
            value: 0,
            width: *width,
        },
        Constant::Float { width, .. } => Constant::Float {
            bits: 0,
            width: *width,
        },
        Constant::Boolean { .. } => Constant::Boolean { value: false },
        _ => Constant::Int {
            value: 0,
            width: 32,
            is_signed: true,
        },
    }
}

/// Build a zero constant for a scalar type.
pub fn constant_zero_for_type(ty: &Type, pointer_width_bits: u16) -> Option<Constant> {
    // handle integer types
    let Some((width, signed)) = ty.int_info_with_pointer_width(pointer_width_bits) else {
        // handle non integer scalar types
        return match ty {
            Type::Float { width } => Some(Constant::Float {
                bits: 0,
                width: *width as u8,
            }),
            Type::Boolean => Some(Constant::Boolean { value: false }),
            _ => None,
        };
    };

    // build integer zero with correct signedness
    if signed {
        Some(Constant::Int {
            value: 0,
            width,
            is_signed: true,
        })
    } else {
        Some(Constant::UInt { value: 0, width })
    }
}

/// Create an all ones constant matching the given constant's type.
pub fn constant_all_ones_like(template: &Constant) -> Constant {
    match template {
        Constant::Int {
            width, is_signed, ..
        } => Constant::Int {
            value: -1,
            width: *width,
            is_signed: *is_signed,
        },
        Constant::UInt { width, .. } => Constant::UInt {
            value: mask_to_width(u128::MAX, *width),
            width: *width,
        },
        Constant::Boolean { .. } => Constant::Boolean { value: true },
        _ => Constant::Int {
            value: -1,
            width: 32,
            is_signed: true,
        },
    }
}

/// Read a scalar constant from an immutable global.
pub fn constant_from_global(
    global: impl Into<mir::GlobalReference>,
    tree: &Tree,
) -> Option<Constant> {
    let global = global.into().global()?;

    // read global definition
    let global = tree.get(global);
    if global.is_mutable() {
        return None;
    }

    // allow scalar initializers only
    match global.initializer.as_ref()? {
        mir::GlobalInitializer::Scalar(constant) => Some(constant.clone()),
        _ => None,
    }
}

/// Constant value tree for aggregate data.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantTree {
    /// Scalar constant value.
    Scalar(Constant),
    /// Aggregate constants in order.
    Aggregate(Vec<ConstantTree>),
    /// Unknown or unsupported constant value.
    Unknown,
}

/// Read a constant tree from an immutable global initializer.
pub fn constant_tree_from_global(
    global: impl Into<mir::GlobalReference>,
    tree: &Tree,
    max_aggregate_elements: usize,
    pointer_width_bits: u16,
) -> Option<ConstantTree> {
    let global = global.into().global()?;

    // read global definition
    let global = tree.get(global);
    if global.is_mutable() {
        return None;
    }

    // read initializer
    let initializer = global.initializer.as_ref()?;

    // build constant tree
    Some(constant_tree_from_initializer(
        initializer,
        global.ty,
        tree,
        max_aggregate_elements,
        pointer_width_bits,
    ))
}

/// Fold a pure intrinsic with constant arguments.
pub fn fold_intrinsic(intrinsic: Intrinsic, arguments: &[Constant]) -> Option<Constant> {
    // reject empty argument lists
    let first = arguments.first()?;

    // fold integer unary intrinsics
    let folded_integer_unary = match intrinsic {
        Intrinsic::LeadingZeroCount => fold_int_unary(first, |value, width| {
            let leading = value.leading_zeros();
            let adjust = u32::from(128u16.saturating_sub(width));
            Some((leading - adjust) as u128)
        }),
        Intrinsic::TrailingZeroCount => {
            fold_int_unary(first, |value, _width| Some(value.trailing_zeros() as u128))
        }
        Intrinsic::PopulationCount => {
            fold_int_unary(first, |value, _width| Some(value.count_ones() as u128))
        }
        Intrinsic::ByteSwap => fold_int_unary(first, |value, width| {
            if width % 8 != 0 {
                return None;
            }
            let swapped = value.swap_bytes();
            Some(mask_to_width(swapped, width))
        }),
        Intrinsic::BitReverse => fold_int_unary(first, |value, width| {
            let reversed = value.reverse_bits();
            let shifted = reversed >> 128u16.saturating_sub(width);
            Some(mask_to_width(shifted, width))
        }),
        _ => None,
    };
    if folded_integer_unary.is_some() {
        return folded_integer_unary;
    }

    // fold integer binary intrinsics
    let folded_integer_binary = match intrinsic {
        Intrinsic::RotateLeft => fold_int_binary(arguments, |value, shift, width| {
            let shift = (shift % u128::from(width)) as u32;
            let rotated = value.rotate_left(shift);
            Some(mask_to_width(rotated, width))
        }),
        Intrinsic::RotateRight => fold_int_binary(arguments, |value, shift, width| {
            let shift = (shift % u128::from(width)) as u32;
            let rotated = value.rotate_right(shift);
            Some(mask_to_width(rotated, width))
        }),
        _ => None,
    };
    if folded_integer_binary.is_some() {
        return folded_integer_binary;
    }

    // fold float unary intrinsics
    let folded_float_unary = match intrinsic {
        Intrinsic::Sqrt => fold_float_unary(first, |value| value.sqrt()),
        Intrinsic::Abs => fold_float_unary(first, |value| value.abs()),
        Intrinsic::Sin => fold_float_unary(first, |value| value.sin()),
        Intrinsic::Cos => fold_float_unary(first, |value| value.cos()),
        Intrinsic::Tan => fold_float_unary(first, |value| value.tan()),
        Intrinsic::Asin => fold_float_unary(first, |value| value.asin()),
        Intrinsic::Acos => fold_float_unary(first, |value| value.acos()),
        Intrinsic::Atan => fold_float_unary(first, |value| value.atan()),
        Intrinsic::Exp => fold_float_unary(first, |value| value.exp()),
        Intrinsic::Exp2 => fold_float_unary(first, |value| value.exp2()),
        Intrinsic::Log => fold_float_unary(first, |value| value.ln()),
        Intrinsic::Log2 => fold_float_unary(first, |value| value.log2()),
        Intrinsic::Log10 => fold_float_unary(first, |value| value.log10()),
        Intrinsic::Floor => fold_float_unary(first, |value| value.floor()),
        Intrinsic::Ceil => fold_float_unary(first, |value| value.ceil()),
        Intrinsic::Trunc => fold_float_unary(first, |value| value.trunc()),
        Intrinsic::Round => fold_float_unary(first, |value| value.round()),
        _ => None,
    };
    if folded_float_unary.is_some() {
        return folded_float_unary;
    }

    // fold float binary and ternary intrinsics
    match intrinsic {
        Intrinsic::CopySign => fold_float_binary(arguments, |left, right| left.copysign(right)),
        Intrinsic::Atan2 => fold_float_binary(arguments, |left, right| left.atan2(right)),
        Intrinsic::Pow => fold_float_binary(arguments, |left, right| left.powf(right)),
        Intrinsic::Fma => fold_float_ternary(arguments, |a, b, c| a.mul_add(b, c)),
        _ => None,
    }
}

/// Fold an integer unary intrinsic when possible.
fn fold_int_unary(
    constant: &Constant,
    f: impl FnOnce(u128, u16) -> Option<u128>,
) -> Option<Constant> {
    // decode the constant payload
    let (value, width, is_signed) = decode_int_constant(constant)?;

    // apply the operation
    let folded = f(value, width)?;

    // reencode with the original signedness
    Some(encode_int_constant(folded, width, is_signed))
}

/// Fold an integer binary intrinsic when possible.
fn fold_int_binary(
    arguments: &[Constant],
    f: impl FnOnce(u128, u128, u16) -> Option<u128>,
) -> Option<Constant> {
    // expect exactly two operands
    let left = arguments.first()?;
    let right = arguments.get(1)?;

    // decode operand payloads
    let (left_value, width, is_signed) = decode_int_constant(left)?;
    let (right_value, right_width, _) = decode_int_constant(right)?;
    if width != right_width {
        return None;
    }

    // apply the operation
    let folded = f(left_value, right_value, width)?;

    // reencode with the original signedness
    Some(encode_int_constant(folded, width, is_signed))
}

/// Fold a float unary intrinsic.
fn fold_float_unary(constant: &Constant, f: impl FnOnce(f64) -> f64) -> Option<Constant> {
    // decode float constant
    let (value, width) = decode_float_constant(constant)?;

    // apply the operation
    let folded = f(value);

    // reencode in the original width
    Some(encode_float_constant(folded, width))
}

/// Fold a float binary intrinsic.
fn fold_float_binary(arguments: &[Constant], f: impl FnOnce(f64, f64) -> f64) -> Option<Constant> {
    // expect exactly two operands
    let left = arguments.first()?;
    let right = arguments.get(1)?;

    // decode operand payloads
    let (left_value, width) = decode_float_constant(left)?;
    let (right_value, right_width) = decode_float_constant(right)?;
    if width != right_width {
        return None;
    }

    // apply the operation
    let folded = f(left_value, right_value);

    // reencode in the original width
    Some(encode_float_constant(folded, width))
}

/// Fold a float ternary intrinsic.
fn fold_float_ternary(
    arguments: &[Constant],
    f: impl FnOnce(f64, f64, f64) -> f64,
) -> Option<Constant> {
    // expect exactly three operands
    let first = arguments.first()?;
    let second = arguments.get(1)?;
    let third = arguments.get(2)?;

    // decode operand payloads
    let (first_value, width) = decode_float_constant(first)?;
    let (second_value, second_width) = decode_float_constant(second)?;
    let (third_value, third_width) = decode_float_constant(third)?;
    if width != second_width || width != third_width {
        return None;
    }

    // apply the operation
    let folded = f(first_value, second_value, third_value);

    // reencode in the original width
    Some(encode_float_constant(folded, width))
}

/// Decode an integer constant into a masked payload.
fn decode_int_constant(constant: &Constant) -> Option<(u128, u16, bool)> {
    match constant {
        Constant::Int {
            value,
            width,
            is_signed,
        } => {
            let masked = mask_to_width(*value as u128, *width);
            Some((masked, *width, *is_signed))
        }
        Constant::UInt { value, width } => Some((*value, *width, false)),
        _ => None,
    }
}

/// Encode a masked integer payload into a constant.
fn encode_int_constant(value: u128, width: u16, is_signed: bool) -> Constant {
    // sign extend when needed
    if is_signed {
        let signed = signed_from_bits(value, width);
        Constant::Int {
            value: signed,
            width,
            is_signed,
        }
    } else {
        Constant::UInt { value, width }
    }
}

/// Decode a float constant into f64 payload and width.
fn decode_float_constant(constant: &Constant) -> Option<(f64, u8)> {
    match constant {
        Constant::Float { bits, width: 32 } => {
            let value = f32::from_bits(*bits as u32) as f64;
            Some((value, 32))
        }
        Constant::Float { bits, width: 64 } => Some((f64::from_bits(*bits), 64)),
        _ => None,
    }
}

/// Encode a float payload back into a constant.
fn encode_float_constant(value: f64, width: u8) -> Constant {
    if width == 32 {
        Constant::Float {
            bits: (value as f32).to_bits() as u64,
            width,
        }
    } else {
        Constant::Float {
            bits: value.to_bits(),
            width,
        }
    }
}

/// Mask an integer payload down to the specified width.
fn mask_to_width(value: u128, width: u16) -> u128 {
    if width >= 128 {
        value
    } else {
        let mask = (1u128 << width) - 1;
        value & mask
    }
}

/// Sign extend a masked integer payload.
fn signed_from_bits(value: u128, width: u16) -> i128 {
    if width >= 128 {
        return value as i128;
    }

    let mask = (1u128 << width) - 1;
    let masked = value & mask;
    let sign_bit = 1u128 << (width - 1);

    if masked & sign_bit != 0 {
        (masked | !mask) as i128
    } else {
        masked as i128
    }
}

/// Build a constant tree from a global initializer.
fn constant_tree_from_initializer(
    initializer: &mir::GlobalInitializer,
    ty: impl Into<mir::TypeReference>,
    tree: &Tree,
    max_aggregate_elements: usize,
    pointer_width_bits: u16,
) -> ConstantTree {
    let Some(ty) = ty.into().ty() else {
        return ConstantTree::Unknown;
    };

    // map initializer kind
    match initializer {
        mir::GlobalInitializer::Zero => {
            constant_tree_from_zero(ty, tree, max_aggregate_elements, pointer_width_bits)
        }
        mir::GlobalInitializer::Scalar(constant) => constant_tree_from_scalar(constant, ty, tree),
        mir::GlobalInitializer::Bytes(bytes) => {
            constant_tree_from_bytes(bytes, ty, tree, max_aggregate_elements)
        }
        mir::GlobalInitializer::Aggregate(elements) => constant_tree_from_aggregate_initializer(
            elements,
            ty,
            tree,
            max_aggregate_elements,
            pointer_width_bits,
        ),
    }
}

/// Build a scalar constant tree when the type is compatible.
fn constant_tree_from_scalar(
    constant: &Constant,
    ty: impl Into<mir::TypeReference>,
    tree: &Tree,
) -> ConstantTree {
    let Some(ty) = ty.into().ty() else {
        return ConstantTree::Unknown;
    };

    // read type
    let ty = tree.get(ty);

    if let Type::Newtype { inner, .. } = ty {
        return constant_tree_from_scalar(constant, *inner, tree);
    }

    // accept scalar types only
    if ty.is_scalar() {
        return ConstantTree::Scalar(constant.clone());
    }

    ConstantTree::Unknown
}

/// Build a zero constant tree for the given type.
fn constant_tree_from_zero(
    ty: impl Into<mir::TypeReference>,
    tree: &Tree,
    max_aggregate_elements: usize,
    pointer_width_bits: u16,
) -> ConstantTree {
    let Some(ty) = ty.into().ty() else {
        return ConstantTree::Unknown;
    };

    // read type
    let ty = tree.get(ty);

    // build zero constants by type
    match ty {
        Type::Boolean => ConstantTree::Scalar(Constant::Boolean { value: false }),
        Type::Int {
            width,
            is_signed: signed,
        } => {
            if *signed {
                ConstantTree::Scalar(Constant::Int {
                    value: 0,
                    width: *width,
                    is_signed: true,
                })
            } else {
                ConstantTree::Scalar(Constant::UInt {
                    value: 0,
                    width: *width,
                })
            }
        }
        Type::Isize => ConstantTree::Scalar(Constant::Int {
            value: 0,
            width: pointer_width_bits,
            is_signed: true,
        }),
        Type::Usize => ConstantTree::Scalar(Constant::UInt {
            value: 0,
            width: pointer_width_bits,
        }),
        Type::Float { width } => ConstantTree::Scalar(Constant::Float {
            bits: 0,
            width: *width as u8,
        }),
        Type::Newtype { inner, .. } => {
            constant_tree_from_zero(*inner, tree, max_aggregate_elements, pointer_width_bits)
        }
        Type::Array {
            element, length, ..
        } => {
            let length = match usize::try_from(*length) {
                Ok(length) => length,
                Err(_) => return ConstantTree::Unknown,
            };

            if length > max_aggregate_elements {
                return ConstantTree::Unknown;
            }

            let element_value =
                constant_tree_from_zero(*element, tree, max_aggregate_elements, pointer_width_bits);
            let elements = (0..length).map(|_| element_value.clone()).collect();
            ConstantTree::Aggregate(elements)
        }
        Type::Tuple { elements, .. } => {
            let elements = elements
                .iter()
                .map(|element| {
                    constant_tree_from_zero(
                        *element,
                        tree,
                        max_aggregate_elements,
                        pointer_width_bits,
                    )
                })
                .collect();
            ConstantTree::Aggregate(elements)
        }
        Type::Struct { fields, .. } => {
            let elements = fields
                .iter()
                .map(|field| tree.get(*field).ty)
                .map(|field_ty| {
                    constant_tree_from_zero(
                        field_ty,
                        tree,
                        max_aggregate_elements,
                        pointer_width_bits,
                    )
                })
                .collect();
            ConstantTree::Aggregate(elements)
        }
        _ => ConstantTree::Unknown,
    }
}

/// Build a constant tree from a byte initializer when possible.
fn constant_tree_from_bytes(
    bytes: &[u8],
    ty: impl Into<mir::TypeReference>,
    tree: &Tree,
    max_aggregate_elements: usize,
) -> ConstantTree {
    let Some(ty) = ty.into().ty() else {
        return ConstantTree::Unknown;
    };

    // read array type
    let Type::Array {
        element, length, ..
    } = tree.get(ty)
    else {
        return ConstantTree::Unknown;
    };

    // check length constraints
    let length = match usize::try_from(*length) {
        Ok(length) => length,
        Err(_) => return ConstantTree::Unknown,
    };

    if length != bytes.len() || length > max_aggregate_elements {
        return ConstantTree::Unknown;
    }

    // require 8 bit integer element type
    let Some(element) = element.ty() else {
        return ConstantTree::Unknown;
    };

    let Type::Int {
        width,
        is_signed: signed,
    } = tree.get(element)
    else {
        return ConstantTree::Unknown;
    };

    if *width != 8 {
        return ConstantTree::Unknown;
    }

    // build per byte constants
    let elements = if *signed {
        bytes
            .iter()
            .map(|byte| {
                let value = i8::from_ne_bytes([*byte]) as i128;
                ConstantTree::Scalar(Constant::Int {
                    value,
                    width: 8,
                    is_signed: true,
                })
            })
            .collect()
    } else {
        bytes
            .iter()
            .map(|byte| {
                ConstantTree::Scalar(Constant::UInt {
                    value: u128::from(*byte),
                    width: 8,
                })
            })
            .collect()
    };

    ConstantTree::Aggregate(elements)
}

/// Build a constant tree from an aggregate initializer.
fn constant_tree_from_aggregate_initializer(
    elements: &[mir::GlobalInitializer],
    ty: impl Into<mir::TypeReference>,
    tree: &Tree,
    max_aggregate_elements: usize,
    pointer_width_bits: u16,
) -> ConstantTree {
    let Some(ty) = ty.into().ty() else {
        return ConstantTree::Unknown;
    };

    // map aggregate initializer to type shape
    match tree.get(ty) {
        Type::Array {
            element, length, ..
        } => {
            let length = match usize::try_from(*length) {
                Ok(length) => length,
                Err(_) => return ConstantTree::Unknown,
            };

            if length != elements.len() || length > max_aggregate_elements {
                return ConstantTree::Unknown;
            }

            let values = elements
                .iter()
                .map(|element_init| {
                    constant_tree_from_initializer(
                        element_init,
                        *element,
                        tree,
                        max_aggregate_elements,
                        pointer_width_bits,
                    )
                })
                .collect();
            ConstantTree::Aggregate(values)
        }
        Type::Tuple {
            elements: element_types,
            ..
        } => {
            if element_types.len() != elements.len() {
                return ConstantTree::Unknown;
            }

            let values = elements
                .iter()
                .zip(element_types)
                .map(|(element_init, element_ty)| {
                    constant_tree_from_initializer(
                        element_init,
                        *element_ty,
                        tree,
                        max_aggregate_elements,
                        pointer_width_bits,
                    )
                })
                .collect();
            ConstantTree::Aggregate(values)
        }
        Type::Struct { fields, .. } => {
            if fields.len() != elements.len() {
                return ConstantTree::Unknown;
            }

            let values = elements
                .iter()
                .zip(fields)
                .map(|(element_init, field)| {
                    constant_tree_from_initializer(
                        element_init,
                        tree.get(*field).ty,
                        tree,
                        max_aggregate_elements,
                        pointer_width_bits,
                    )
                })
                .collect();
            ConstantTree::Aggregate(values)
        }
        _ => ConstantTree::Unknown,
    }
}

/// Try to fold a binary operation on constants.
pub fn fold_binary(operator: BinaryOperator, left: Constant, right: Constant) -> Option<Constant> {
    match (&left, &right) {
        (
            Constant::Int {
                value: l,
                width: lw,
                is_signed: true,
            },
            Constant::Int {
                value: r,
                width: rw,
                is_signed: true,
            },
        ) if lw == rw => fold_binary_signed(*l, *r, *lw, operator),

        (
            Constant::UInt {
                value: l,
                width: lw,
            },
            Constant::UInt {
                value: r,
                width: rw,
            },
        ) if lw == rw => fold_binary_unsigned(*l, *r, *lw, operator),

        (
            Constant::Float {
                bits: lb,
                width: lw,
            },
            Constant::Float {
                bits: rb,
                width: rw,
            },
        ) if lw == rw => fold_binary_float(*lb, *rb, *lw, operator),

        (Constant::Boolean { value: l }, Constant::Boolean { value: r }) => {
            fold_binary_bool(*l, *r, operator)
        }

        _ => None,
    }
}

/// Fold a binary operation on signed integers.
pub fn fold_binary_signed(
    left: i128,
    right: i128,
    width: u16,
    operator: BinaryOperator,
) -> Option<Constant> {
    let result_int = |value: i128| {
        Some(Constant::Int {
            value: truncate_signed(value, width),
            width,
            is_signed: true,
        })
    };
    let result_bool = |value: bool| Some(Constant::Boolean { value });

    match operator {
        BinaryOperator::Add => result_int(left.wrapping_add(right)),
        BinaryOperator::Subtract => result_int(left.wrapping_sub(right)),
        BinaryOperator::Multiply => result_int(left.wrapping_mul(right)),
        BinaryOperator::SignedDivide => {
            if right != 0 {
                result_int(left.wrapping_div(right))
            } else {
                None
            }
        }
        BinaryOperator::SignedRemainder => {
            if right != 0 {
                result_int(left.wrapping_rem(right))
            } else {
                None
            }
        }
        BinaryOperator::And => result_int(left & right),
        BinaryOperator::Or => result_int(left | right),
        BinaryOperator::Xor => result_int(left ^ right),
        BinaryOperator::ShiftLeft => result_int(left.wrapping_shl(right as u32)),
        BinaryOperator::ArithmeticShiftRight => result_int(left.wrapping_shr(right as u32)),
        BinaryOperator::Equal => result_bool(left == right),
        BinaryOperator::NotEqual => result_bool(left != right),
        BinaryOperator::SignedLessThan => result_bool(left < right),
        BinaryOperator::SignedLessEqual => result_bool(left <= right),
        BinaryOperator::SignedGreaterThan => result_bool(left > right),
        BinaryOperator::SignedGreaterEqual => result_bool(left >= right),
        _ => None,
    }
}

/// Fold a binary operation on unsigned integers.
pub fn fold_binary_unsigned(
    left: u128,
    right: u128,
    width: u16,
    operator: BinaryOperator,
) -> Option<Constant> {
    let result_uint = |value: u128| {
        Some(Constant::UInt {
            value: mask_to_width(value, width),
            width,
        })
    };
    let result_bool = |value: bool| Some(Constant::Boolean { value });

    match operator {
        BinaryOperator::Add => result_uint(left.wrapping_add(right)),
        BinaryOperator::Subtract => result_uint(left.wrapping_sub(right)),
        BinaryOperator::Multiply => result_uint(left.wrapping_mul(right)),
        BinaryOperator::UnsignedDivide => {
            if right != 0 {
                result_uint(left.wrapping_div(right))
            } else {
                None
            }
        }
        BinaryOperator::UnsignedRemainder => {
            if right != 0 {
                result_uint(left.wrapping_rem(right))
            } else {
                None
            }
        }
        BinaryOperator::And => result_uint(left & right),
        BinaryOperator::Or => result_uint(left | right),
        BinaryOperator::Xor => result_uint(left ^ right),
        BinaryOperator::ShiftLeft => result_uint(left.wrapping_shl(right as u32)),
        BinaryOperator::LogicalShiftRight => result_uint(left.wrapping_shr(right as u32)),
        BinaryOperator::Equal => result_bool(left == right),
        BinaryOperator::NotEqual => result_bool(left != right),
        BinaryOperator::UnsignedLessThan => result_bool(left < right),
        BinaryOperator::UnsignedLessEqual => result_bool(left <= right),
        BinaryOperator::UnsignedGreaterThan => result_bool(left > right),
        BinaryOperator::UnsignedGreaterEqual => result_bool(left >= right),
        _ => None,
    }
}

/// Fold a binary operation on floats.
pub fn fold_binary_float(
    left_bits: u64,
    right_bits: u64,
    width: u8,
    operator: BinaryOperator,
) -> Option<Constant> {
    let result_float = |value: f64| {
        Some(Constant::Float {
            bits: value.to_bits(),
            width,
        })
    };
    let result_bool = |value: bool| Some(Constant::Boolean { value });

    if width == 32 {
        let left = f32::from_bits(left_bits as u32);
        let right = f32::from_bits(right_bits as u32);
        match operator {
            BinaryOperator::FloatAdd => result_float((left + right) as f64),
            BinaryOperator::FloatSubtract => result_float((left - right) as f64),
            BinaryOperator::FloatMultiply => result_float((left * right) as f64),
            BinaryOperator::FloatDivide => result_float((left / right) as f64),
            BinaryOperator::FloatEqual => result_bool(left == right),
            BinaryOperator::FloatNotEqual => result_bool(left != right),
            BinaryOperator::FloatLessThan => result_bool(left < right),
            BinaryOperator::FloatLessEqual => result_bool(left <= right),
            BinaryOperator::FloatGreaterThan => result_bool(left > right),
            BinaryOperator::FloatGreaterEqual => result_bool(left >= right),
            _ => None,
        }
    } else if width == 64 {
        let left = f64::from_bits(left_bits);
        let right = f64::from_bits(right_bits);
        match operator {
            BinaryOperator::FloatAdd => result_float(left + right),
            BinaryOperator::FloatSubtract => result_float(left - right),
            BinaryOperator::FloatMultiply => result_float(left * right),
            BinaryOperator::FloatDivide => result_float(left / right),
            BinaryOperator::FloatEqual => result_bool(left == right),
            BinaryOperator::FloatNotEqual => result_bool(left != right),
            BinaryOperator::FloatLessThan => result_bool(left < right),
            BinaryOperator::FloatLessEqual => result_bool(left <= right),
            BinaryOperator::FloatGreaterThan => result_bool(left > right),
            BinaryOperator::FloatGreaterEqual => result_bool(left >= right),
            _ => None,
        }
    } else {
        None
    }
}

/// Fold a binary operation on booleans.
pub fn fold_binary_bool(left: bool, right: bool, operator: BinaryOperator) -> Option<Constant> {
    let result_bool = |value: bool| Some(Constant::Boolean { value });

    match operator {
        BinaryOperator::And => result_bool(left && right),
        BinaryOperator::Or => result_bool(left || right),
        BinaryOperator::Xor => result_bool(left ^ right),
        BinaryOperator::Equal => result_bool(left == right),
        BinaryOperator::NotEqual => result_bool(left != right),
        _ => None,
    }
}

/// Try to fold a cast operation on a constant.
pub fn fold_cast(
    operator: CastOperator,
    value: Constant,
    to_type: LocalNodeId<Type>,
    pointer_width_bits: u16,
    tree: &Tree,
) -> Option<Constant> {
    // load target type
    let target_type = tree.get(to_type);

    // apply cast semantics
    match operator {
        CastOperator::Bitcast => Some(value),

        CastOperator::Truncate => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => return Some(value),
            };

            // truncate integer values
            match value {
                Constant::Int { value, .. } => Some(Constant::Int {
                    value: truncate_signed(value, target_width),
                    width: target_width,
                    is_signed: true,
                }),
                Constant::UInt { value, .. } => Some(Constant::UInt {
                    value: truncate_unsigned(value, target_width),
                    width: target_width,
                }),
                _ => Some(value),
            }
        }

        CastOperator::ZeroExtend => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => return Some(value),
            };

            // zero extend integer values
            match value {
                Constant::UInt { value, .. } => Some(Constant::UInt {
                    value,
                    width: target_width,
                }),
                Constant::Int { value, width, .. } => {
                    let masked = truncate_unsigned(value as u128, width);
                    Some(Constant::UInt {
                        value: masked,
                        width: target_width,
                    })
                }
                _ => Some(value),
            }
        }

        CastOperator::SignExtend => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => return Some(value),
            };

            // sign extend integer values
            match value {
                Constant::Int { value, width, .. } => Some(Constant::Int {
                    value: sign_extend(value, width, target_width),
                    width: target_width,
                    is_signed: true,
                }),
                Constant::UInt { value, width } => {
                    let as_signed = signed_from_bits(value, width);
                    Some(Constant::Int {
                        value: sign_extend(as_signed, width, target_width),
                        width: target_width,
                        is_signed: true,
                    })
                }
                _ => Some(value),
            }
        }

        CastOperator::FloatToSignedInt => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => 64,
            };

            // convert float to signed int
            match value {
                Constant::Float { bits, width: 32 } => {
                    let value = f32::from_bits(bits as u32) as f64;
                    let (min_bound, max_bound) = integer_bounds(target_width, true)?;
                    let converted = float_to_int_checked(value, min_bound, max_bound)?;
                    Some(Constant::Int {
                        value: converted,
                        width: target_width,
                        is_signed: true,
                    })
                }
                Constant::Float { bits, width: 64 } => {
                    let value = f64::from_bits(bits);
                    let (min_bound, max_bound) = integer_bounds(target_width, true)?;
                    let converted = float_to_int_checked(value, min_bound, max_bound)?;
                    Some(Constant::Int {
                        value: converted,
                        width: target_width,
                        is_signed: true,
                    })
                }
                _ => Some(value),
            }
        }

        CastOperator::FloatToUnsignedInt => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => 64,
            };

            // convert float to unsigned int
            match value {
                Constant::Float { bits, width: 32 } => {
                    let value = f32::from_bits(bits as u32) as f64;
                    let (min_bound, max_bound) = integer_bounds(target_width, false)?;
                    let converted = float_to_int_checked(value, min_bound, max_bound)?;
                    Some(Constant::UInt {
                        value: converted as u128,
                        width: target_width,
                    })
                }
                Constant::Float { bits, width: 64 } => {
                    let value = f64::from_bits(bits);
                    let (min_bound, max_bound) = integer_bounds(target_width, false)?;
                    let converted = float_to_int_checked(value, min_bound, max_bound)?;
                    Some(Constant::UInt {
                        value: converted as u128,
                        width: target_width,
                    })
                }
                _ => Some(value),
            }
        }

        CastOperator::FloatToSignedIntSaturating => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => 64,
            };

            // convert float to signed int with saturation
            match value {
                Constant::Float { bits, width: 32 } => {
                    let value = f32::from_bits(bits as u32) as f64;
                    let (min_bound, max_bound) = integer_bounds(target_width, true)?;
                    let converted = float_to_int_saturating(value, min_bound, max_bound);
                    Some(Constant::Int {
                        value: converted,
                        width: target_width,
                        is_signed: true,
                    })
                }
                Constant::Float { bits, width: 64 } => {
                    let value = f64::from_bits(bits);
                    let (min_bound, max_bound) = integer_bounds(target_width, true)?;
                    let converted = float_to_int_saturating(value, min_bound, max_bound);
                    Some(Constant::Int {
                        value: converted,
                        width: target_width,
                        is_signed: true,
                    })
                }
                _ => Some(value),
            }
        }

        CastOperator::FloatToUnsignedIntSaturating => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => 64,
            };

            // convert float to unsigned int with saturation
            match value {
                Constant::Float { bits, width: 32 } => {
                    let value = f32::from_bits(bits as u32) as f64;
                    let (min_bound, max_bound) = integer_bounds(target_width, false)?;
                    let converted = float_to_int_saturating(value, min_bound, max_bound);
                    Some(Constant::UInt {
                        value: converted as u128,
                        width: target_width,
                    })
                }
                Constant::Float { bits, width: 64 } => {
                    let value = f64::from_bits(bits);
                    let (min_bound, max_bound) = integer_bounds(target_width, false)?;
                    let converted = float_to_int_saturating(value, min_bound, max_bound);
                    Some(Constant::UInt {
                        value: converted as u128,
                        width: target_width,
                    })
                }
                _ => Some(value),
            }
        }

        CastOperator::SignedIntToFloat => {
            // read target float width
            let target_width = match target_type {
                Type::Float { width } => *width,
                _ => 64,
            };

            // convert signed int to float
            match value {
                Constant::Int { value, .. } if target_width == 32 => Some(Constant::Float {
                    bits: (value as f32).to_bits() as u64,
                    width: 32,
                }),
                Constant::Int { value, .. } => Some(Constant::Float {
                    bits: (value as f64).to_bits(),
                    width: 64,
                }),
                _ => Some(value),
            }
        }

        CastOperator::UnsignedIntToFloat => {
            // read target float width
            let target_width = match target_type {
                Type::Float { width } => *width,
                _ => 64,
            };

            // convert unsigned int to float
            match value {
                Constant::UInt { value, .. } if target_width == 32 => Some(Constant::Float {
                    bits: (value as f32).to_bits() as u64,
                    width: 32,
                }),
                Constant::UInt { value, .. } => Some(Constant::Float {
                    bits: (value as f64).to_bits(),
                    width: 64,
                }),
                _ => Some(value),
            }
        }

        CastOperator::FloatTruncate => {
            // truncate float64 to float32
            match value {
                Constant::Float { bits, width: 64 } => {
                    let f = f64::from_bits(bits);
                    Some(Constant::Float {
                        bits: (f as f32).to_bits() as u64,
                        width: 32,
                    })
                }
                _ => Some(value),
            }
        }

        CastOperator::FloatExtend => {
            // extend float32 to float64
            match value {
                Constant::Float { bits, width: 32 } => {
                    let f = f32::from_bits(bits as u32);
                    Some(Constant::Float {
                        bits: (f as f64).to_bits(),
                        width: 64,
                    })
                }
                _ => Some(value),
            }
        }

        CastOperator::PointerToInt | CastOperator::IntToPointer => None,
    }
}

/// Truncate a signed integer to a target bit width.
fn truncate_signed(value: i128, width: u16) -> i128 {
    // handle full width
    if width >= 128 {
        return value;
    }

    // build bit mask
    let mask = (1u128 << width) - 1;
    let masked = (value as u128) & mask;
    let sign_bit = 1u128 << (width - 1);

    // set sign extension when needed
    if masked & sign_bit != 0 {
        (masked | !mask) as i128
    }
    // otherwise keep masked value
    else {
        masked as i128
    }
}

/// Compute integer bounds for a width and signedness.
fn integer_bounds(width: u16, is_signed: bool) -> Option<(i128, i128)> {
    if width == 0 || width > 128 || (!is_signed && width == 128) {
        return None;
    }

    if is_signed {
        let shift = (width - 1) as u32;
        let min = -(1_i128 << shift);
        let max = (1_i128 << shift) - 1;
        Some((min, max))
    } else {
        let shift = width as u32;
        let max = (1_i128 << shift) - 1;
        Some((0, max))
    }
}

/// Convert a float to an integer when the conversion is in range and finite.
fn float_to_int_checked(value: f64, min_bound: i128, max_bound: i128) -> Option<i128> {
    if !value.is_finite() {
        return None;
    }

    let min_float = min_bound as f64;
    let max_float = max_bound as f64;
    let max_rounded = max_float.trunc() as i128;
    let max_is_rounded_up = max_rounded > max_bound;

    let min_ok = value >= min_float;
    let max_ok = if max_is_rounded_up {
        value < max_float
    } else {
        value <= max_float
    };

    if !min_ok || !max_ok {
        return None;
    }

    let truncated = value.trunc() as i128;
    if truncated < min_bound || truncated > max_bound {
        return None;
    }

    Some(truncated)
}

/// Convert a float to an integer with saturation.
fn float_to_int_saturating(value: f64, min_bound: i128, max_bound: i128) -> i128 {
    if value.is_nan() {
        return 0;
    }

    if !value.is_finite() {
        return if value.is_sign_negative() {
            min_bound
        } else {
            max_bound
        };
    }

    let min_float = min_bound as f64;
    let max_float = max_bound as f64;
    let max_rounded = max_float.trunc() as i128;
    let max_is_rounded_up = max_rounded > max_bound;

    if value <= min_float {
        return min_bound;
    }

    if max_is_rounded_up {
        if value >= max_float {
            return max_bound;
        }
    } else if value >= max_float {
        return max_bound;
    }

    let truncated = value.trunc() as i128;
    if truncated < min_bound {
        return min_bound;
    }

    if truncated > max_bound {
        return max_bound;
    }

    truncated
}

/// Truncate an unsigned integer to a target bit width.
fn truncate_unsigned(value: u128, width: u16) -> u128 {
    mask_to_width(value, width)
}

/// Sign extend a value from one width to another.
fn sign_extend(value: i128, from_width: u16, to_width: u16) -> i128 {
    // handle no extend case
    if from_width >= to_width || from_width >= 128 {
        return value;
    }

    // extend using truncate logic
    truncate_signed(value, from_width)
}

/// Try to fold a unary operation on a constant.
pub fn fold_unary(operator: UnaryOperator, value: Constant) -> Option<Constant> {
    match (operator, &value) {
        (
            UnaryOperator::Negate,
            Constant::Int {
                value: v,
                width,
                is_signed: true,
            },
        ) => Some(Constant::Int {
            value: v.wrapping_neg(),
            width: *width,
            is_signed: true,
        }),

        (UnaryOperator::FloatNegate, Constant::Float { bits, width }) => {
            if *width == 32 {
                let f = f32::from_bits(*bits as u32);
                Some(Constant::Float {
                    bits: ((-f).to_bits()) as u64,
                    width: 32,
                })
            } else if *width == 64 {
                let f = f64::from_bits(*bits);
                Some(Constant::Float {
                    bits: (-f).to_bits(),
                    width: 64,
                })
            } else {
                None
            }
        }

        (UnaryOperator::Not, Constant::Boolean { value: v }) => {
            Some(Constant::Boolean { value: !v })
        }

        (
            UnaryOperator::Not,
            Constant::Int {
                value: v,
                width,
                is_signed,
            },
        ) => Some(Constant::Int {
            value: !v,
            width: *width,
            is_signed: *is_signed,
        }),

        (UnaryOperator::Not, Constant::UInt { value: v, width }) => Some(Constant::UInt {
            value: !v,
            width: *width,
        }),

        _ => None,
    }
}

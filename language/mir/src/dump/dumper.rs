//! MIR tree dumper for debugging and visualization.

use crate::{
    BinaryOperator, Block, CastKind, Constant, Function, Global, GlobalInitializer, Instruction,
    Local, LocalNodeId, Mutability, NodeTree, NodeVisitor, NodeVisitorOptions, Ownership,
    SwitchCase, Terminator, Type, UnaryOperator, Value,
};
use destack_source::{Color, StringPool};

/// Options for the MIR dumper.
#[derive(Debug, Clone, Copy)]
pub struct DumperOptions {
    /// Use colors in output.
    pub use_colors: bool,
    /// Show node IDs.
    pub show_ids: bool,
}

impl Default for DumperOptions {
    fn default() -> Self {
        Self {
            use_colors: true,
            show_ids: true,
        }
    }
}

/// A dumper for MIR trees.
#[derive(Debug)]
pub struct Dumper<'a> {
    /// The node tree.
    tree: &'a NodeTree,
    /// The string pool.
    strings: &'a StringPool,
    /// The dump options.
    options: DumperOptions,
    /// The visitor options.
    visitor_options: NodeVisitorOptions,
    /// The buffer we're writing to.
    buffer: String,
    /// The current indent level.
    indent: usize,
}

impl<'a> Dumper<'a> {
    /// Create a new dumper.
    pub fn new(tree: &'a NodeTree, strings: &'a StringPool, options: DumperOptions) -> Self {
        Self {
            tree,
            strings,
            options,
            visitor_options: NodeVisitorOptions::default(),
            buffer: String::new(),
            indent: 0,
        }
    }

    /// Dump all globals and functions in the tree.
    pub fn dump_all(&mut self) {
        // dump globals first
        for (id, global) in self.tree.iter_nodes::<Global>() {
            self.visit_global(self.tree, id, global);
        }
        // then functions
        for (id, function) in self.tree.iter_nodes::<Function>() {
            self.visit_function(self.tree, id, function);
        }
    }

    /// Finish and return the dumped string.
    pub fn finish(self) -> String {
        self.buffer
    }

    // === writing helpers ===

    fn write(&mut self, s: &str) {
        self.buffer.push_str(s);
    }

    fn write_colored(&mut self, s: &str, color: Color) {
        if self.options.use_colors {
            self.buffer.push_str(&color.apply(s));
        } else {
            self.buffer.push_str(s);
        }
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.buffer.push_str("  ");
        }
    }

    fn indent(&mut self) {
        self.indent += 1;
    }

    fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    // === value formatting ===

    fn format_value(&self, v: Value) -> String {
        format!("v{}", v.0)
    }

    fn format_block_id(&self, id: LocalNodeId<Block>) -> String {
        format!("block{}", id.id)
    }

    fn format_local_id(&self, id: LocalNodeId<Local>) -> String {
        format!("local{}", id.id)
    }

    fn format_global_id(&self, id: LocalNodeId<Global>) -> String {
        let global = self.tree.get(id);
        let name = self.strings.get(global.name).to_string();
        format!("@{name}")
    }

    fn format_function_id(&self, id: LocalNodeId<Function>) -> String {
        let function = self.tree.get(id);
        let name = self.strings.get(function.name).to_string();
        format!("@{name}")
    }

    fn format_type_id(&self, id: LocalNodeId<Type>) -> String {
        let ty = self.tree.get(id);
        self.format_type(ty)
    }

    fn format_type(&self, ty: &Type) -> String {
        match ty {
            Type::Void => "void".to_string(),
            Type::Boolean => "bool".to_string(),
            Type::Int { width, signed } => {
                if *signed {
                    format!("i{width}")
                } else {
                    format!("u{width}")
                }
            }
            Type::Float { width } => format!("f{width}"),
            Type::RawPointer { .. } => "rawptr".to_string(),
            Type::ManagedReference {
                nullable: false, ..
            } => "ref".to_string(),
            Type::ManagedReference { nullable: true, .. } => "ref?".to_string(),
            Type::Array { length, .. } => format!("[_; {length}]"),
            Type::Tuple { elements } => format!("({})", elements.len()),
            Type::Struct { fields } => format!("struct{{{}}}", fields.len()),
            Type::FunctionPointer { parameters, .. } => format!("fn({})", parameters.len()),
        }
    }

    fn format_constant(&self, c: &Constant) -> String {
        match c {
            Constant::Boolean { value } => format!("{value}"),
            Constant::Int {
                value,
                width,
                is_signed,
            } => {
                if *is_signed {
                    format!("{value}i{width}")
                } else {
                    format!("{value}u{width}")
                }
            }
            Constant::UInt { value, width } => format!("{value}u{width}"),
            Constant::Float { bits, width } => {
                if *width == 32 {
                    format!("{}f32", f32::from_bits(*bits as u32))
                } else {
                    format!("{}f64", f64::from_bits(*bits))
                }
            }
            Constant::String { value } => format!("{value:?}"),
            Constant::Char { value } => format!("{value:?}"),
        }
    }

    fn format_binary_op(&self, op: BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::Add => "add",
            BinaryOperator::Subtract => "sub",
            BinaryOperator::Multiply => "mul",
            BinaryOperator::SignedDivide => "sdiv",
            BinaryOperator::UnsignedDivide => "udiv",
            BinaryOperator::SignedRemainder => "srem",
            BinaryOperator::UnsignedRemainder => "urem",
            BinaryOperator::FloatAdd => "fadd",
            BinaryOperator::FloatSubtract => "fsub",
            BinaryOperator::FloatMultiply => "fmul",
            BinaryOperator::FloatDivide => "fdiv",
            BinaryOperator::And => "and",
            BinaryOperator::Or => "or",
            BinaryOperator::Xor => "xor",
            BinaryOperator::ShiftLeft => "shl",
            BinaryOperator::ArithmeticShiftRight => "ashr",
            BinaryOperator::LogicalShiftRight => "lshr",
            BinaryOperator::Equal => "eq",
            BinaryOperator::NotEqual => "ne",
            BinaryOperator::SignedLessThan => "slt",
            BinaryOperator::SignedLessEqual => "sle",
            BinaryOperator::SignedGreaterThan => "sgt",
            BinaryOperator::SignedGreaterEqual => "sge",
            BinaryOperator::UnsignedLessThan => "ult",
            BinaryOperator::UnsignedLessEqual => "ule",
            BinaryOperator::UnsignedGreaterThan => "ugt",
            BinaryOperator::UnsignedGreaterEqual => "uge",
            BinaryOperator::FloatEqual => "feq",
            BinaryOperator::FloatNotEqual => "fne",
            BinaryOperator::FloatLessThan => "flt",
            BinaryOperator::FloatLessEqual => "fle",
            BinaryOperator::FloatGreaterThan => "fgt",
            BinaryOperator::FloatGreaterEqual => "fge",
        }
    }

    fn format_unary_op(&self, op: UnaryOperator) -> &'static str {
        match op {
            UnaryOperator::Negate => "neg",
            UnaryOperator::FloatNegate => "fneg",
            UnaryOperator::Not => "not",
        }
    }

    fn format_cast_kind(&self, kind: CastKind) -> &'static str {
        match kind {
            CastKind::Bitcast => "bitcast",
            CastKind::Truncate => "trunc",
            CastKind::ZeroExtend => "zext",
            CastKind::SignExtend => "sext",
            CastKind::FloatToSignedInt => "fptosi",
            CastKind::FloatToUnsignedInt => "fptoui",
            CastKind::SignedIntToFloat => "sitofp",
            CastKind::UnsignedIntToFloat => "uitofp",
            CastKind::FloatTruncate => "fptrunc",
            CastKind::FloatExtend => "fpext",
            CastKind::PointerToInt => "ptrtoint",
            CastKind::IntToPointer => "inttoptr",
        }
    }

    // === instruction dumping ===

    fn dump_instruction(&mut self, inst: &Instruction) {
        self.write_indent();

        match inst {
            Instruction::Constant { destination, value } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = const ");
                self.write_colored(&self.format_constant(value), Color::Yellow);
            }

            Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = ");
                self.write_colored(self.format_binary_op(*operator), Color::Cyan);
                self.write(" ");
                self.write(&self.format_value(*left));
                self.write(", ");
                self.write(&self.format_value(*right));
            }

            Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = ");
                self.write_colored(self.format_unary_op(*operator), Color::Cyan);
                self.write(" ");
                self.write(&self.format_value(*argument));
            }

            Instruction::Cast {
                destination,
                kind,
                argument,
                to_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = ");
                self.write_colored(self.format_cast_kind(*kind), Color::Cyan);
                self.write(" ");
                self.write(&self.format_value(*argument));
                self.write(" to ");
                self.write_colored(&self.format_type_id(*to_type), Color::Magenta);
            }

            Instruction::LocalGet { destination, local } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = local_get ");
                self.write(&self.format_local_id(*local));
            }

            Instruction::LocalSet { local, value } => {
                self.write("local_set ");
                self.write(&self.format_local_id(*local));
                self.write(", ");
                self.write(&self.format_value(*value));
            }

            Instruction::GlobalGet {
                destination,
                global,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = global_get ");
                self.write(&self.format_global_id(*global));
            }

            Instruction::GlobalSet { global, value } => {
                self.write("global_set ");
                self.write(&self.format_global_id(*global));
                self.write(", ");
                self.write(&self.format_value(*value));
            }

            Instruction::Load {
                destination,
                pointer,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = load ");
                self.write(&self.format_value(*pointer));
            }

            Instruction::Store { pointer, value } => {
                self.write("store ");
                self.write(&self.format_value(*pointer));
                self.write(", ");
                self.write(&self.format_value(*value));
            }

            Instruction::ExtractField {
                destination,
                aggregate,
                index,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = extractfield ");
                self.write(&self.format_value(*aggregate));
                self.write(&format!(", {index}"));
            }

            Instruction::InsertField {
                destination,
                aggregate,
                index,
                value,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = insertfield ");
                self.write(&self.format_value(*aggregate));
                self.write(&format!(", {index}, "));
                self.write(&self.format_value(*value));
            }

            Instruction::ExtractElement {
                destination,
                array,
                index,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = extractelement ");
                self.write(&self.format_value(*array));
                self.write(", ");
                self.write(&self.format_value(*index));
            }

            Instruction::InsertElement {
                destination,
                array,
                index,
                value,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = insertelement ");
                self.write(&self.format_value(*array));
                self.write(", ");
                self.write(&self.format_value(*index));
                self.write(", ");
                self.write(&self.format_value(*value));
            }

            Instruction::Call {
                destination,
                function,
                arguments,
            } => {
                if let Some(dst) = destination {
                    self.write_colored(&self.format_value(*dst), Color::Green);
                    self.write(" = ");
                }
                self.write("call ");
                self.write(&self.format_function_id(*function));
                self.write("(");
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
            }

            Instruction::CallIndirect {
                destination,
                callee,
                arguments,
            } => {
                if let Some(dst) = destination {
                    self.write_colored(&self.format_value(*dst), Color::Green);
                    self.write(" = ");
                }
                self.write("call_indirect ");
                self.write(&self.format_value(*callee));
                self.write("(");
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
            }

            Instruction::ManagedAllocate {
                destination,
                layout,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = managed_allocate ");
                self.write_colored(&self.format_type_id(*layout), Color::Magenta);
            }

            Instruction::ManagedAllocateArray {
                destination,
                element,
                length,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = managed_allocate_array ");
                self.write_colored(&self.format_type_id(*element), Color::Magenta);
                self.write(", ");
                self.write(&self.format_value(*length));
            }

            Instruction::RawAllocate {
                destination,
                layout,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = raw_allocate ");
                self.write_colored(&self.format_type_id(*layout), Color::Magenta);
            }

            Instruction::RawFree { pointer } => {
                self.write("raw_free ");
                self.write(&self.format_value(*pointer));
            }

            Instruction::StackAllocate {
                destination,
                layout,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = stack_allocate ");
                self.write_colored(&self.format_type_id(*layout), Color::Magenta);
            }

            Instruction::Drop { value } => {
                self.write("drop ");
                self.write(&self.format_value(*value));
            }
        }

        self.write("\n");
    }

    // === terminator dumping ===

    fn dump_terminator(&mut self, term: &Terminator) {
        self.write_indent();
        match term {
            Terminator::Return { value } => {
                self.write_colored("return", Color::Red);
                if let Some(v) = value {
                    self.write(" ");
                    self.write(&self.format_value(*v));
                }
            }

            Terminator::Jump { target, arguments } => {
                self.write_colored("jump", Color::Red);
                self.write(" ");
                self.write(&self.format_block_id(*target));
                if !arguments.is_empty() {
                    self.write("(");
                    for (i, arg) in arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write(&self.format_value(*arg));
                    }
                    self.write(")");
                }
            }

            Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => {
                self.write_colored("branch", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*condition));
                self.write(", ");
                self.write(&self.format_block_id(*then_target));
                if !then_arguments.is_empty() {
                    self.write("(");
                    for (i, arg) in then_arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write(&self.format_value(*arg));
                    }
                    self.write(")");
                }
                self.write(", ");
                self.write(&self.format_block_id(*else_target));
                if !else_arguments.is_empty() {
                    self.write("(");
                    for (i, arg) in else_arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write(&self.format_value(*arg));
                    }
                    self.write(")");
                }
            }

            Terminator::Switch {
                value,
                default,
                default_arguments,
                cases,
            } => {
                self.write_colored("switch", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*value));
                self.write(", default ");
                self.write(&self.format_block_id(*default));
                if !default_arguments.is_empty() {
                    self.write("(");
                    for (i, arg) in default_arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write(&self.format_value(*arg));
                    }
                    self.write(")");
                }
                for SwitchCase {
                    value: case_val,
                    target,
                    arguments,
                } in cases
                {
                    self.write(&format!(", {case_val} => "));
                    self.write(&self.format_block_id(*target));
                    if !arguments.is_empty() {
                        self.write("(");
                        for (i, arg) in arguments.iter().enumerate() {
                            if i > 0 {
                                self.write(", ");
                            }
                            self.write(&self.format_value(*arg));
                        }
                        self.write(")");
                    }
                }
            }

            Terminator::Unreachable => {
                self.write_colored("unreachable", Color::Red);
            }
        }
        self.write("\n");
    }
}

impl<'a> NodeVisitor for Dumper<'a> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.visitor_options
    }

    fn visit_function(&mut self, tree: &NodeTree, id: LocalNodeId<Function>, function: &Function) {
        // function header
        self.write_colored("function", Color::BrightBlue);
        self.write(" @");
        self.write(&format!("function{}", id.id));
        self.write("(");

        // parameters
        for (i, param) in function.parameters.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&self.format_value(param.value));
            self.write(": ");
            self.write_colored(&self.format_type_id(param.ty), Color::Magenta);
        }
        self.write(") -> ");
        self.write_colored(&self.format_type_id(function.return_type), Color::Magenta);
        self.write(" {\n");

        self.indent();

        // locals
        if !function.locals.is_empty() {
            for local_id in &function.locals {
                let local = tree.get(*local_id);
                self.write_indent();
                self.write(&self.format_local_id(*local_id));
                self.write(": ");
                self.write_colored(&self.format_type_id(local.ty), Color::Magenta);
                match local.mutability {
                    Mutability::Mutable => self.write(" (var)"),
                    Mutability::Immutable => self.write(" (const)"),
                }
                match local.ownership {
                    Ownership::Owned => self.write(" [owned]"),
                    Ownership::Borrowed => self.write(" [borrowed]"),
                    Ownership::Copy => self.write(" [copy]"),
                }
                self.write("\n");
            }
            self.write("\n");
        }

        // blocks
        for block_id in &function.blocks {
            let block = tree.get(*block_id);
            self.visit_block(tree, *block_id, block);
        }

        self.dedent();
        self.write("}\n\n");
    }

    fn visit_block(&mut self, tree: &NodeTree, id: LocalNodeId<Block>, block: &Block) {
        // block label
        self.write_indent();
        self.write_colored(&self.format_block_id(id), Color::Yellow);

        // block parameters
        if !block.parameters.is_empty() {
            self.write("(");
            for (i, param) in block.parameters.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&self.format_value(param.value));
                self.write(": ");
                self.write_colored(&self.format_type_id(param.ty), Color::Magenta);
            }
            self.write(")");
        }
        self.write(":\n");

        self.indent();

        // instructions
        for inst_id in &block.instructions {
            let instruction = tree.get(*inst_id);
            self.dump_instruction(instruction);
        }

        // terminator
        self.dump_terminator(&block.terminator);

        self.dedent();
    }

    fn visit_instruction(
        &mut self,
        _tree: &NodeTree,
        _id: LocalNodeId<Instruction>,
        instruction: &Instruction,
    ) {
        self.dump_instruction(instruction);
    }

    fn visit_local(&mut self, _tree: &NodeTree, _id: LocalNodeId<Local>, _local: &Local) {
        // nothing to do
    }

    fn visit_type(&mut self, _tree: &NodeTree, _id: LocalNodeId<Type>, _ty: &Type) {
        // nothing to do
    }

    fn visit_global(&mut self, _tree: &NodeTree, id: LocalNodeId<Global>, global: &Global) {
        if global.is_external {
            self.write_colored("extern ", Color::BrightBlue);
        }
        self.write_colored("global", Color::BrightBlue);
        self.write(" @");
        self.write(&format!("global{}", id.id));
        self.write(": ");
        self.write_colored(&self.format_type_id(global.ty), Color::Magenta);
        if let Some(init) = &global.initializer {
            self.write(" = ");
            self.dump_data_init(init);
        }
        match global.mutability {
            Mutability::Mutable => self.write(" ; var"),
            Mutability::Immutable => self.write(" ; const"),
        }
        self.write("\n");
    }
}

impl<'a> Dumper<'a> {
    /// Dump a data initializer.
    fn dump_data_init(&mut self, init: &GlobalInitializer) {
        match init {
            GlobalInitializer::Zero => self.write("zeroinit"),
            GlobalInitializer::Scalar(constant) => self.write(&self.format_constant(constant)),
            GlobalInitializer::Bytes(bytes) => {
                self.write("\"");
                for byte in bytes {
                    self.write(&format!("\\x{byte:02x}"));
                }
                self.write("\"");
            }
            GlobalInitializer::Aggregate(elements) => {
                self.write("{");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.dump_data_init(elem);
                }
                self.write("}");
            }
        }
    }
}

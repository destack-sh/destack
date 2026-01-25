use crate::{
    AddressSpace, BinaryOperator, Block, CastOperator, CheckConstraint, Constant, Function, Global,
    GlobalInitializer, Instruction, Local, LocalNodeId, MemoryLocationSet, MemorySemantics,
    Mutability, NodeTree, NodeVisitor, NodeVisitorOptions, Ownership, ReferenceKind, SwitchCase,
    Terminator, Type, UnaryOperator, Value,
};
use destack_base::{Color, StringPool};

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
            Type::Int {
                width,
                is_signed: signed,
            } => {
                if *signed {
                    format!("i{width}")
                } else {
                    format!("u{width}")
                }
            }
            Type::Isize => "isize".to_string(),
            Type::Usize => "usize".to_string(),
            Type::Float { width } => format!("f{width}"),
            Type::Type => "type".to_string(),
            Type::Reference {
                kind,
                address_space,
                mutability,
                is_nullable,
                ..
            } => {
                let ref_prefix = if *is_nullable { "ref?" } else { "ref" };
                let kind_label = match kind {
                    ReferenceKind::Managed => "managed",
                    ReferenceKind::Owned => "owned",
                    ReferenceKind::Borrowed => "borrowed",
                    ReferenceKind::Raw => "raw",
                };

                // address space label
                let address_space_label = match address_space {
                    AddressSpace::Generic => None,
                    AddressSpace::Target(id) => Some(id.to_string()),
                    _ => address_space.keyword().map(|name| name.to_string()),
                };
                let address_space_label = address_space_label
                    .map(|label| format!(" addrspace({label})"))
                    .unwrap_or_default();
                let mutability_label = match mutability {
                    Mutability::Mutable => " mut",
                    Mutability::Immutable => "",
                };
                format!("{ref_prefix}<{kind_label}{address_space_label}{mutability_label}>")
            }
            Type::Array { length, .. } => format!("[_; {length}]"),
            Type::Tuple {
                elements,
                copyability: _,
            } => format!("({})", elements.len()),
            Type::Struct {
                fields,
                copyability: _,
            } => format!("struct{{{}}}", fields.len()),
            Type::Vector { lanes, .. } => format!("vector<{lanes}>"),
            Type::Tensor { shape, .. } => format!("tensor<{}>", shape.len()),
            Type::TensorReference { shape, .. } => format!("tensor_ref<{}>", shape.len()),
            Type::FunctionPointer { parameters, .. } => format!("fn({})", parameters.len()),
        }
    }

    fn format_memory_semantics(&self, semantics: MemorySemantics) -> String {
        // collect location names
        let mut names = self.collect_memory_location_names(semantics.locations);

        // append semantics flags
        if semantics.is_volatile {
            names.push("volatile");
        }
        if semantics.is_make_available {
            names.push("make_available");
        }
        if semantics.is_make_visible {
            names.push("make_visible");
        }

        // render as a single token or list
        if names.len() == 1 {
            names[0].to_string()
        } else {
            format!("[{}]", names.join(", "))
        }
    }

    fn collect_memory_location_names(&self, locations: MemoryLocationSet) -> Vec<&'static str> {
        // handle named location sets
        if locations == MemoryLocationSet::NONE {
            return vec!["none"];
        }
        if locations == MemoryLocationSet::ANY {
            return vec!["any"];
        }

        // collect ordered locations
        let ordered = [
            ("arguments", MemoryLocationSet::ARGUMENTS),
            ("heap", MemoryLocationSet::HEAP),
            ("stack", MemoryLocationSet::STACK),
            ("global", MemoryLocationSet::GLOBAL),
            ("shared", MemoryLocationSet::SHARED),
            ("local", MemoryLocationSet::LOCAL),
            ("constant", MemoryLocationSet::CONSTANT),
            ("inaccessible", MemoryLocationSet::INACCESSIBLE),
            ("io", MemoryLocationSet::IO),
        ];
        let mut names = Vec::new();
        for (name, set) in ordered {
            if locations.contains(set) {
                names.push(name);
            }
        }
        names
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

    fn format_cast_operator(&self, operator: CastOperator) -> &'static str {
        match operator {
            CastOperator::Bitcast => "bitcast",
            CastOperator::Truncate => "trunc",
            CastOperator::ZeroExtend => "zext",
            CastOperator::SignExtend => "sext",
            CastOperator::FloatToSignedInt => "fptosi",
            CastOperator::FloatToUnsignedInt => "fptoui",
            CastOperator::SignedIntToFloat => "sitofp",
            CastOperator::UnsignedIntToFloat => "uitofp",
            CastOperator::FloatTruncate => "fptrunc",
            CastOperator::FloatExtend => "fpext",
            CastOperator::PointerToInt => "ptrtoint",
            CastOperator::IntToPointer => "inttoptr",
        }
    }

    // === instruction dumping ===

    fn dump_instruction(&mut self, inst: &Instruction) {
        self.write_indent();

        match inst {
            Instruction::Const { destination, value } => {
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
                operator,
                argument,
                to_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = ");
                self.write_colored(self.format_cast_operator(*operator), Color::Cyan);
                self.write(" ");
                self.write(&self.format_value(*argument));
                self.write(" to ");
                self.write_colored(&self.format_type_id(*to_type), Color::Magenta);
            }

            Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = ");
                self.write_colored("select", Color::Cyan);
                self.write(" ");
                self.write(&self.format_value(*condition));
                self.write(", ");
                self.write(&self.format_value(*then_value));
                self.write(", ");
                self.write(&self.format_value(*else_value));
            }

            Instruction::LocalGet { destination, local } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = local.get ");
                self.write(&self.format_local_id(*local));
            }

            Instruction::LocalAddr {
                destination,
                local,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = local.addr ");
                self.write(&self.format_local_id(*local));
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*result_type), Color::Magenta);
            }

            Instruction::LocalSet { local, value } => {
                self.write("local.set ");
                self.write(&self.format_local_id(*local));
                self.write(", ");
                self.write(&self.format_value(*value));
            }

            Instruction::GlobalAddr {
                destination,
                global,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = global.addr ");
                self.write(&self.format_global_id(*global));
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*result_type), Color::Magenta);
            }

            Instruction::GlobalConst {
                destination,
                global,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = global.const ");
                self.write(&self.format_global_id(*global));
            }

            Instruction::FunctionAddr {
                destination,
                function,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = function.addr ");
                self.write(&self.format_function_id(*function));
            }

            Instruction::Load {
                destination,
                pointer,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = load ");
                self.write(&self.format_value(*pointer));
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*result_type), Color::Magenta);
            }

            Instruction::Store { pointer, value } => {
                self.write("store ");
                self.write(&self.format_value(*pointer));
                self.write(", ");
                self.write(&self.format_value(*value));
            }

            Instruction::RawDrop { value } => {
                self.write("raw.drop ");
                self.write(&self.format_value(*value));
            }

            Instruction::StackDrop { value } => {
                self.write("stack.drop ");
                self.write(&self.format_value(*value));
            }

            Instruction::Assume { condition } => {
                self.write("assume ");
                self.write(&self.format_value(*condition));
            }

            Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = field.get ");
                self.write(&self.format_value(*aggregate));
                self.write(&format!(", {index}"));
            }

            Instruction::FieldAddr {
                destination,
                aggregate,
                index,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = field.addr ");
                self.write(&self.format_value(*aggregate));
                self.write(&format!(", {index}"));
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*result_type), Color::Magenta);
            }

            Instruction::FieldSet {
                destination,
                aggregate,
                index,
                value,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = field.set ");
                self.write(&self.format_value(*aggregate));
                self.write(&format!(", {index}, "));
                self.write(&self.format_value(*value));
            }

            Instruction::ElementGet {
                destination,
                array,
                index,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = element.get ");
                self.write(&self.format_value(*array));
                self.write(", ");
                self.write(&self.format_value(*index));
            }

            Instruction::ElementAddr {
                destination,
                array,
                index,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = element.addr ");
                self.write(&self.format_value(*array));
                self.write(", ");
                self.write(&self.format_value(*index));
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*result_type), Color::Magenta);
            }

            Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = element.set ");
                self.write(&self.format_value(*array));
                self.write(", ");
                self.write(&self.format_value(*index));
                self.write(", ");
                self.write(&self.format_value(*value));
            }

            Instruction::Struct {
                destination,
                ty,
                fields,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = struct ");
                self.write_colored(&self.format_type_id(*ty), Color::Magenta);
                self.write("(");
                let args = self.tree.get_arguments(*fields);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
            }

            Instruction::Tuple {
                destination,
                ty,
                elements,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tuple ");
                self.write_colored(&self.format_type_id(*ty), Color::Magenta);
                self.write("(");
                let args = self.tree.get_arguments(*elements);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
            }

            Instruction::Array {
                destination,
                ty,
                elements,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = array ");
                self.write_colored(&self.format_type_id(*ty), Color::Magenta);
                self.write("(");
                let args = self.tree.get_arguments(*elements);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
            }

            Instruction::VectorSplat { destination, value } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = vector.splat ");
                self.write(&self.format_value(*value));
            }

            Instruction::VectorExtract {
                destination,
                vector,
                index,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = vector.extract ");
                self.write(&self.format_value(*vector));
                self.write(", ");
                self.write(&self.format_value(*index));
            }

            Instruction::VectorInsert {
                destination,
                vector,
                index,
                value,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = vector.insert ");
                self.write(&self.format_value(*vector));
                self.write(", ");
                self.write(&self.format_value(*index));
                self.write(", ");
                self.write(&self.format_value(*value));
            }

            Instruction::VectorShuffle {
                destination,
                left,
                right,
                mask,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = vector.shuffle ");
                self.write(&self.format_value(*left));
                self.write(", ");
                self.write(&self.format_value(*right));
                self.write(", [");
                for (i, lane) in mask.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&lane.to_string());
                }
                self.write("]");
            }

            Instruction::VectorReduce {
                destination,
                operator,
                vector,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = vector.reduce ");
                self.write(operator.to_str());
                self.write(", ");
                self.write(&self.format_value(*vector));
            }
            Instruction::VectorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = vector.compare ");
                self.write(operator.to_str());
                self.write(", ");
                self.write(&self.format_value(*left));
                self.write(", ");
                self.write(&self.format_value(*right));
            }
            Instruction::VectorConvert {
                destination,
                mode,
                vector,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = vector.convert ");
                self.write(mode.to_str());
                self.write(", ");
                self.write(&self.format_value(*vector));
            }

            Instruction::TensorLoad {
                destination,
                view,
                indices,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.load ");
                self.write(&self.format_value(*view));
                self.write(", [");
                let args = self.tree.get_arguments(*indices);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write("]");
            }

            Instruction::TensorStore {
                view,
                indices,
                value,
            } => {
                self.write("tensor.store ");
                self.write(&self.format_value(*view));
                self.write(", [");
                let args = self.tree.get_arguments(*indices);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write("], ");
                self.write(&self.format_value(*value));
            }

            Instruction::TensorFill { view, value } => {
                self.write("tensor.fill ");
                self.write(&self.format_value(*view));
                self.write(", ");
                self.write(&self.format_value(*value));
            }

            Instruction::TensorCopy { target, source } => {
                self.write("tensor.copy ");
                self.write(&self.format_value(*target));
                self.write(", ");
                self.write(&self.format_value(*source));
            }

            Instruction::TensorReshape {
                destination,
                tensor,
                shape,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.reshape ");
                self.write(&self.format_value(*tensor));
                let args = self.tree.get_arguments(*shape);
                if !args.is_empty() {
                    self.write(", [");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write(&self.format_value(*arg));
                    }
                    self.write("]");
                }
            }

            Instruction::TensorBroadcast {
                destination,
                tensor,
                dimensions,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.broadcast ");
                self.write(&self.format_value(*tensor));
                self.write(", [");
                for (i, dim) in dimensions.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("]");
            }

            Instruction::TensorTranspose {
                destination,
                tensor,
                permutation,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.transpose ");
                self.write(&self.format_value(*tensor));
                self.write(", [");
                for (i, dim) in permutation.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("]");
            }
            Instruction::TensorCast {
                destination,
                tensor,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.cast ");
                self.write(&self.format_value(*tensor));
            }
            Instruction::TensorView {
                destination,
                view,
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.view ");
                self.write(&self.format_value(*view));
                self.write(", offsets=[");
                let args = self.tree.get_arguments(*arguments);
                let (offsets, sizes, strides) =
                    split_tensor_ranges(args, *offsets_count, *sizes_count, *strides_count);
                for (i, offset) in offsets.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*offset));
                }
                self.write("], sizes=[");
                for (i, size) in sizes.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*size));
                }
                self.write("], strides=[");
                for (i, stride) in strides.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*stride));
                }
                self.write("]");
            }

            Instruction::TensorSlice {
                destination,
                tensor,
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.slice ");
                self.write(&self.format_value(*tensor));
                self.write(", offsets=[");
                let args = self.tree.get_arguments(*arguments);
                let offsets_end = *offsets_count as usize;
                let sizes_end = offsets_end + *sizes_count as usize;
                for (i, arg) in args.iter().take(offsets_end).enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write("], sizes=[");
                for (i, arg) in args
                    .iter()
                    .skip(offsets_end)
                    .take(*sizes_count as usize)
                    .enumerate()
                {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write("], strides=[");
                for (i, arg) in args
                    .iter()
                    .skip(sizes_end)
                    .take(*strides_count as usize)
                    .enumerate()
                {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write("]");
            }

            Instruction::TensorPad {
                destination,
                tensor,
                arguments,
                low_count,
                high_count,
                interior_count,
                value,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.pad ");
                self.write(&self.format_value(*tensor));
                self.write(", value=");
                self.write(&self.format_value(*value));
                let args = self.tree.get_arguments(*arguments);
                let low_end = *low_count as usize;
                let high_end = low_end + *high_count as usize;
                self.write(", low=[");
                for (i, arg) in args.iter().take(low_end).enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write("], high=[");
                for (i, arg) in args
                    .iter()
                    .skip(low_end)
                    .take(*high_count as usize)
                    .enumerate()
                {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write("], interior=[");
                for (i, arg) in args
                    .iter()
                    .skip(high_end)
                    .take(*interior_count as usize)
                    .enumerate()
                {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write("]");
            }

            Instruction::TensorConcat {
                destination,
                tensors,
                axis,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.concat [");
                let args = self.tree.get_arguments(*tensors);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write("], axis=");
                self.write(&axis.to_string());
            }

            Instruction::TensorReduce {
                destination,
                operator,
                tensor,
                initial,
                axes,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.reduce ");
                self.write(operator.to_str());
                self.write(", ");
                self.write(&self.format_value(*tensor));
                self.write(", ");
                self.write(&self.format_value(*initial));
                self.write(", axes=[");
                for (i, axis) in axes.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&axis.to_string());
                }
                self.write("]");
            }

            Instruction::TensorDot {
                destination,
                left,
                right,
                dimensions,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.dot ");
                self.write(&self.format_value(*left));
                self.write(", ");
                self.write(&self.format_value(*right));
                self.write(", dims(lhs_batch=[");
                for (i, dim) in dimensions.lhs_batch.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], rhs_batch=[");
                for (i, dim) in dimensions.rhs_batch.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], lhs_contract=[");
                for (i, dim) in dimensions.lhs_contracting.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], rhs_contract=[");
                for (i, dim) in dimensions.rhs_contracting.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("])");
            }

            Instruction::TensorConvolution {
                destination,
                input,
                kernel,
                dimensions,
                window,
                feature_group_count,
                batch_group_count,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.convolution ");
                self.write(&self.format_value(*input));
                self.write(", ");
                self.write(&self.format_value(*kernel));
                self.write(", dims(input_batch=");
                self.write(&dimensions.input_batch.to_string());
                self.write(", input_feature=");
                self.write(&dimensions.input_feature.to_string());
                self.write(", input_spatial=[");
                for (i, dim) in dimensions.input_spatial.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], kernel_input_feature=");
                self.write(&dimensions.kernel_input_feature.to_string());
                self.write(", kernel_output_feature=");
                self.write(&dimensions.kernel_output_feature.to_string());
                self.write(", kernel_spatial=[");
                for (i, dim) in dimensions.kernel_spatial.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], output_batch=");
                self.write(&dimensions.output_batch.to_string());
                self.write(", output_feature=");
                self.write(&dimensions.output_feature.to_string());
                self.write(", output_spatial=[");
                for (i, dim) in dimensions.output_spatial.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("])");
                self.write(", strides=[");
                for (i, stride) in window.strides.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&stride.to_string());
                }
                self.write("], padding_low=[");
                for (i, pad) in window.padding_low.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&pad.to_string());
                }
                self.write("], padding_high=[");
                for (i, pad) in window.padding_high.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&pad.to_string());
                }
                self.write("], lhs_dilation=[");
                for (i, dilation) in window.lhs_dilation.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dilation.to_string());
                }
                self.write("], rhs_dilation=[");
                for (i, dilation) in window.rhs_dilation.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dilation.to_string());
                }
                self.write("], window_reversal=[");
                for (i, reverse) in window.window_reversal.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(if *reverse { "true" } else { "false" });
                }
                self.write("], feature_group=");
                self.write(&feature_group_count.to_string());
                self.write(", batch_group=");
                self.write(&batch_group_count.to_string());
            }

            Instruction::TensorGather {
                destination,
                operand,
                indices,
                dimensions,
                slice_sizes,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.gather ");
                self.write(&self.format_value(*operand));
                self.write(", ");
                self.write(&self.format_value(*indices));
                self.write(", dims(offset_dims=[");
                for (i, dim) in dimensions.offset_dims.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], collapsed_slice_dims=[");
                for (i, dim) in dimensions.collapsed_slice_dims.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], start_index_map=[");
                for (i, dim) in dimensions.start_index_map.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], index_vector_dim=");
                self.write(&dimensions.index_vector_dim.to_string());
                self.write("), slice_sizes=[");
                for (i, size) in slice_sizes.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&size.to_string());
                }
                self.write("]");
            }

            Instruction::TensorScatter {
                destination,
                operand,
                indices,
                updates,
                dimensions,
                mode,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.scatter ");
                self.write(&self.format_value(*operand));
                self.write(", ");
                self.write(&self.format_value(*indices));
                self.write(", ");
                self.write(&self.format_value(*updates));
                self.write(", dims(update_window_dims=[");
                for (i, dim) in dimensions.update_window_dims.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], inserted_window_dims=[");
                for (i, dim) in dimensions.inserted_window_dims.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], scatter_dims_to_operand_dims=[");
                for (i, dim) in dimensions.scatter_dims_to_operand_dims.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("], index_vector_dim=");
                self.write(&dimensions.index_vector_dim.to_string());
                self.write("), mode=");
                self.write(mode.to_str());
            }

            Instruction::TensorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.compare ");
                self.write(operator.to_str());
                self.write(", ");
                self.write(&self.format_value(*left));
                self.write(", ");
                self.write(&self.format_value(*right));
            }
            Instruction::TensorConvert {
                destination,
                mode,
                tensor,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.convert ");
                self.write(mode.to_str());
                self.write(", ");
                self.write(&self.format_value(*tensor));
            }

            Instruction::Call {
                destination,
                function,
                arguments,
                signature,
                ..
            } => {
                if let Some(dst) = destination {
                    self.write_colored(&self.format_value(*dst), Color::Green);
                    self.write(" = ");
                }
                self.write("call ");
                self.write(&self.format_function_id(*function));
                self.write("(");
                let args = self.tree.get_arguments(*arguments);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*signature), Color::Magenta);
            }

            Instruction::CallVirtual {
                destination,
                receiver,
                arguments,
                declaring_type,
                slot_id,
                declared_target,
                signature,
                ..
            } => {
                if let Some(dst) = destination {
                    self.write_colored(&self.format_value(*dst), Color::Green);
                    self.write(" = ");
                }
                self.write("call.virtual ");
                self.write(&self.format_value(*receiver));
                self.write(", ");
                self.write_colored(&self.format_type_id(*declaring_type), Color::Magenta);
                self.write(", ");
                self.write(&slot_id.to_string());
                if let Some(target) = declared_target {
                    self.write(", ");
                    self.write(&self.format_function_id(*target));
                }
                self.write("(");
                let args = self.tree.get_arguments(*arguments);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*signature), Color::Magenta);
            }

            Instruction::CallInterface {
                destination,
                receiver,
                arguments,
                declaring_type,
                slot_id,
                declared_target,
                signature,
                ..
            } => {
                if let Some(dst) = destination {
                    self.write_colored(&self.format_value(*dst), Color::Green);
                    self.write(" = ");
                }
                self.write("call.interface ");
                self.write(&self.format_value(*receiver));
                self.write(", ");
                self.write_colored(&self.format_type_id(*declaring_type), Color::Magenta);
                self.write(", ");
                self.write(&slot_id.to_string());
                if let Some(target) = declared_target {
                    self.write(", ");
                    self.write(&self.format_function_id(*target));
                }
                self.write("(");
                let args = self.tree.get_arguments(*arguments);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*signature), Color::Magenta);
            }

            Instruction::CallIndirect {
                destination,
                callee,
                arguments,
                signature,
                ..
            } => {
                if let Some(dst) = destination {
                    self.write_colored(&self.format_value(*dst), Color::Green);
                    self.write(" = ");
                }
                self.write("call.indirect ");
                self.write(&self.format_value(*callee));
                self.write("(");
                let args = self.tree.get_arguments(*arguments);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*signature), Color::Magenta);
            }

            Instruction::ManagedAlloc {
                destination,
                layout,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = managed.alloc ");
                self.write_colored(&self.format_type_id(*layout), Color::Magenta);
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*result_type), Color::Magenta);
            }

            Instruction::ManagedAllocArray {
                destination,
                element,
                length,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = managed.alloc_array ");
                self.write_colored(&self.format_type_id(*element), Color::Magenta);
                self.write(", ");
                self.write(&self.format_value(*length));
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*result_type), Color::Magenta);
            }

            Instruction::RawAlloc {
                destination,
                layout,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = raw.alloc ");
                self.write_colored(&self.format_type_id(*layout), Color::Magenta);
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*result_type), Color::Magenta);
            }

            Instruction::RawFree { pointer } => {
                self.write("raw.free ");
                self.write(&self.format_value(*pointer));
            }

            Instruction::StackAlloc {
                destination,
                layout,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = stack.alloc ");
                self.write_colored(&self.format_type_id(*layout), Color::Magenta);
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*result_type), Color::Magenta);
            }

            Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
                ordering,
                scope,
                memory_scope,
                semantics,
            } => {
                if let Some(dst) = destination {
                    self.write_colored(&self.format_value(*dst), Color::Green);
                    self.write(" = ");
                }
                self.write_colored(&format!("intrinsic.{}", intrinsic.to_str()), Color::Cyan);
                self.write("(");
                let args = self.tree.get_arguments(*arguments);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                if let Some(ord) = ordering {
                    if !args.is_empty() {
                        self.write(", ");
                    }
                    self.write_colored(&format!("ordering={}", ord.to_str()), Color::Yellow);
                }
                if let Some(scope) = scope {
                    if !args.is_empty() || ordering.is_some() {
                        self.write(", ");
                    }
                    self.write_colored(&format!("scope={}", scope.to_str()), Color::Yellow);
                }
                if let Some(memory_scope) = memory_scope {
                    if !args.is_empty() || ordering.is_some() || scope.is_some() {
                        self.write(", ");
                    }
                    self.write_colored(
                        &format!("memory_scope={}", memory_scope.to_str()),
                        Color::Yellow,
                    );
                }
                if let Some(semantics) = semantics {
                    if !args.is_empty()
                        || ordering.is_some()
                        || scope.is_some()
                        || memory_scope.is_some()
                    {
                        self.write(", ");
                    }
                    self.write_colored("semantics=", Color::Yellow);
                    self.write_colored(&self.format_memory_semantics(*semantics), Color::Yellow);
                }
                self.write(")");
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

            Terminator::Check {
                condition,
                constraint,
                success,
                failure,
            } => {
                self.write_colored("check", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*condition));
                self.write(", ");
                match constraint {
                    CheckConstraint::Bounds {
                        index,
                        length,
                        collection,
                        is_signed,
                    } => {
                        let prefix = if *is_signed {
                            "bounds.signed"
                        } else {
                            "bounds.unsigned"
                        };
                        self.write(prefix);
                        self.write(" ");
                        self.write(&self.format_value(*index));
                        self.write(", ");
                        self.write(&self.format_value(*length));
                        self.write(", ");
                        self.write(&self.format_value(*collection));
                    }
                    CheckConstraint::Null { value } => {
                        self.write("null ");
                        self.write(&self.format_value(*value));
                    }
                    CheckConstraint::DivZero { divisor } => {
                        self.write("div_zero ");
                        self.write(&self.format_value(*divisor));
                    }
                    CheckConstraint::Type { value, expected } => {
                        self.write("type ");
                        self.write(&self.format_value(*value));
                        self.write(", ");
                        self.write(&self.format_type_id(*expected));
                    }
                    CheckConstraint::Union { value, expected } => {
                        self.write("union ");
                        self.write(&self.format_value(*value));
                        self.write(", ");
                        self.write(&expected.to_string());
                    }
                    CheckConstraint::Vtable { receiver, expected } => {
                        self.write("vtable ");
                        self.write(&self.format_value(*receiver));
                        self.write(", ");
                        self.write(&self.format_type_id(*expected));
                    }
                    CheckConstraint::Itab { receiver, expected } => {
                        self.write("itab ");
                        self.write(&self.format_value(*receiver));
                        self.write(", ");
                        self.write(&expected.index().to_string());
                    }
                    CheckConstraint::ShiftRange {
                        value,
                        bit_width,
                        is_signed,
                    } => {
                        let prefix = if *is_signed {
                            "shift.signed"
                        } else {
                            "shift.unsigned"
                        };
                        self.write(prefix);
                        self.write(" ");
                        self.write(&self.format_value(*value));
                        self.write(", ");
                        self.write(&bit_width.to_string());
                    }
                    CheckConstraint::Narrow {
                        value,
                        to_width,
                        is_signed,
                    } => {
                        let prefix = if *is_signed {
                            "narrow.signed"
                        } else {
                            "narrow.unsigned"
                        };
                        self.write(prefix);
                        self.write(" ");
                        self.write(&self.format_value(*value));
                        self.write(", ");
                        self.write(&to_width.to_string());
                    }
                    CheckConstraint::Overflow {
                        operator,
                        left,
                        right,
                        is_signed,
                    } => {
                        let prefix = if *is_signed {
                            "overflow.signed"
                        } else {
                            "overflow.unsigned"
                        };
                        self.write(&format!("{prefix}.{}", operator.to_str()));
                        self.write(" ");
                        self.write(&self.format_value(*left));
                        self.write(", ");
                        self.write(&self.format_value(*right));
                    }
                }
                self.write(", ");
                self.write(&self.format_block_id(success.target));
                if !success.arguments.is_empty() {
                    self.write("(");
                    for (i, arg) in success.arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write(&self.format_value(*arg));
                    }
                    self.write(")");
                }
                self.write(", ");
                self.write(&self.format_block_id(failure.target));
                if !failure.arguments.is_empty() {
                    self.write("(");
                    for (i, arg) in failure.arguments.iter().enumerate() {
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

            Terminator::Yield {
                value,
                resume,
                resume_arguments,
            } => {
                self.write_colored("yield", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*value));
                self.write(", ");
                self.write(&self.format_block_id(*resume));
                if !resume_arguments.is_empty() {
                    self.write("(");
                    for (i, arg) in resume_arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write(&self.format_value(*arg));
                    }
                    self.write(")");
                }
            }

            Terminator::TailCall {
                function,
                arguments,
            } => {
                self.write_colored("tailcall", Color::Red);
                self.write(" ");
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

            Terminator::TailCallIndirect {
                callee,
                arguments,
                signature,
            } => {
                self.write_colored("tailcall.indirect", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*callee));
                self.write("(");
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*signature), Color::Magenta);
            }

            Terminator::TailCallVirtual {
                receiver,
                arguments,
                declaring_type,
                slot_id,
                declared_target,
                signature,
            } => {
                self.write_colored("tailcall.virtual", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*receiver));
                self.write(", ");
                self.write_colored(&self.format_type_id(*declaring_type), Color::Magenta);
                self.write(", ");
                self.write(&slot_id.to_string());
                if let Some(target) = declared_target {
                    self.write(", ");
                    self.write(&self.format_function_id(*target));
                }
                self.write("(");
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*signature), Color::Magenta);
            }

            Terminator::TailCallInterface {
                receiver,
                arguments,
                declaring_type,
                slot_id,
                declared_target,
                signature,
            } => {
                self.write_colored("tailcall.interface", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*receiver));
                self.write(", ");
                self.write_colored(&self.format_type_id(*declaring_type), Color::Magenta);
                self.write(", ");
                self.write(&slot_id.to_string());
                if let Some(target) = declared_target {
                    self.write(", ");
                    self.write(&self.format_function_id(*target));
                }
                self.write("(");
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*signature), Color::Magenta);
            }
        }
        self.write("\n");
    }
}

/// Split a packed tensor range list into offsets, sizes, and strides.
fn split_tensor_ranges(
    values: &[Value],
    offsets_count: u16,
    sizes_count: u16,
    strides_count: u16,
) -> (&[Value], &[Value], &[Value]) {
    // compute slice bounds
    let offsets_end = offsets_count as usize;
    let sizes_end = offsets_end + sizes_count as usize;
    let strides_end = sizes_end + strides_count as usize;

    // slice the packed range list
    let offsets = &values[..offsets_end];
    let sizes = &values[offsets_end..sizes_end];
    let strides = &values[sizes_end..strides_end];

    (offsets, sizes, strides)
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
                    Mutability::Mutable => self.write(" (mut)"),
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
        if global.linkage.is_import() {
            self.write_colored("extern ", Color::BrightBlue);
        } else if global.linkage.is_exported() {
            self.write_colored("export ", Color::BrightBlue);
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
            Mutability::Mutable => self.write(" ; mut"),
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
            GlobalInitializer::String(value) => {
                self.write("\"");
                for ch in value.chars() {
                    if ch == '"' {
                        self.write("\\\"");
                    } else if ch == '\\' {
                        self.write("\\\\");
                    } else if ch == '\n' {
                        self.write("\\n");
                    } else if ch == '\r' {
                        self.write("\\r");
                    } else if ch == '\t' {
                        self.write("\\t");
                    } else if ch.is_ascii_graphic() || ch == ' ' {
                        self.write(&ch.to_string());
                    } else {
                        self.write(&format!("\\u{{{:x}}}", ch as u32));
                    }
                }
                self.write("\"");
            }
            GlobalInitializer::Bytes(bytes) => {
                self.write("b\"");
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

use crate::{
    BinaryOperator, Block, BlockReference, BlockTarget, CastOperator, CheckConstraint, Constant,
    Function, FunctionReference, Global, GlobalInitializer, GlobalReference, Instruction, Local,
    LocalNodeId, LocalReference, MemoryRegionSet, MemorySemantics, Mutability, NodeTree,
    NodeVisitor, NodeVisitorOptions, Ownership, ReferenceKind, SwitchCase, Terminator, TrapKind,
    Type, TypeReference, UnaryOperator, ValueReference,
};
use destack_core::{Color, StringPool};

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

    fn format_value<V>(&self, value: V) -> String
    where
        V: Into<ValueReference>,
    {
        match value.into() {
            ValueReference::Value(value) => format!("v{}", value.0),
            ValueReference::Missing => "<missing>".to_string(),
            ValueReference::Error => "<error>".to_string(),
        }
    }

    fn format_block_id(&self, id: LocalNodeId<Block>) -> String {
        format!("b{}", id.id)
    }

    fn format_block_reference(&self, block: BlockReference) -> String {
        match block {
            BlockReference::Block(block) => self.format_block_id(block),
            BlockReference::Missing => "<missing>".to_string(),
            BlockReference::Error => "<error>".to_string(),
        }
    }

    fn format_block_target(&self, target: &BlockTarget) -> String {
        self.format_block_reference(target.block)
    }

    fn write_block_target(&mut self, target: &BlockTarget) {
        self.write(&self.format_block_target(target));

        if target.arguments.is_empty() {
            return;
        }

        self.write("(");
        for (index, argument) in target.arguments.iter().enumerate() {
            if index > 0 {
                self.write(", ");
            }

            self.write(&self.format_value(*argument));
        }
        self.write(")");
    }

    fn format_local_id<L>(&self, local: L) -> String
    where
        L: Into<LocalReference>,
    {
        match local.into() {
            LocalReference::Local(local) => format!("local{}", local.id),
            LocalReference::Missing => "<missing>".to_string(),
            LocalReference::Error => "<error>".to_string(),
        }
    }

    fn format_global_id<G>(&self, global: G) -> String
    where
        G: Into<GlobalReference>,
    {
        match global.into() {
            GlobalReference::Global(global) => {
                let global = self.tree.get(global);
                self.strings.get(global.name).to_string()
            }
            GlobalReference::Missing => "<missing>".to_string(),
            GlobalReference::Error => "<error>".to_string(),
        }
    }

    fn format_function_id<F>(&self, function: F) -> String
    where
        F: Into<FunctionReference>,
    {
        match function.into() {
            FunctionReference::Function(function) => {
                let function = self.tree.get(function);
                self.strings.get(function.name).to_string()
            }
            FunctionReference::Missing => "<missing>".to_string(),
            FunctionReference::Error => "<error>".to_string(),
        }
    }

    fn format_type_id<T>(&self, ty: T) -> String
    where
        T: Into<TypeReference>,
    {
        match ty.into() {
            TypeReference::Type(ty) => {
                if let Some(name) = self.tree.type_display_name(ty) {
                    return self.strings.get(name).to_string();
                }

                let ty = self.tree.get(ty);
                self.format_type(ty)
            }
            TypeReference::Missing => "<missing>".to_string(),
            TypeReference::Error => "<error>".to_string(),
        }
    }

    fn format_type(&self, ty: &Type) -> String {
        match ty {
            Type::Void => "void".to_string(),
            Type::Boolean => "boolean".to_string(),
            Type::Int {
                width,
                is_signed: signed,
            } => {
                if *signed {
                    format!("int{width}")
                } else {
                    format!("uint{width}")
                }
            }
            Type::Isize => "isize".to_string(),
            Type::Usize => "usize".to_string(),
            Type::Float { width } => format!("float{width}"),
            Type::TypeDescriptor => "typeDescriptor".to_string(),
            Type::TypeId => "typeId".to_string(),
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
                let address_space_label = if address_space.is_local() {
                    None
                } else {
                    Some(address_space.label().to_string())
                };
                let address_space_label = address_space_label
                    .map(|label| format!(", space({label})"))
                    .unwrap_or_default();
                let mutability_label = match mutability {
                    Mutability::Mutable => "",
                    Mutability::Immutable => ", readonly",
                };
                format!("{ref_prefix}<_, {kind_label}{mutability_label}{address_space_label}>")
            }
            Type::Array { length, .. } => format!("_[{length}]"),
            Type::Slice { .. } => "slice<_>".to_string(),
            Type::Tuple { elements, copy: _ } => format!("({})", elements.len()),
            Type::Struct { fields, copy: _ } => format!("struct{{{}}}", fields.len()),
            Type::Newtype { .. } => "newtype".to_string(),
            Type::Vector { lanes, .. } => format!("vector<{lanes}>"),
            Type::Tensor { shape, .. } => format!("tensor<{}>", shape.len()),
            Type::TensorView { shape, .. } => format!("tensorView<{}>", shape.len()),
            Type::FunctionSignature { parameters, result } => {
                let parameter_count = parameters.len();
                let result = self.format_type_id(*result);
                format!("sig(/* {parameter_count} */) -> {result}")
            }
            Type::FunctionPointer { signature } => {
                let signature = self.format_type_id(*signature);
                format!("fn({signature})")
            }
            Type::Closure { signature } => {
                let signature = self.format_type_id(*signature);
                format!("({signature}) => <?>")
            }
        }
    }

    fn format_memory_semantics(&self, semantics: MemorySemantics) -> String {
        // collect location names
        let mut names = self.collect_effect_region_names(semantics.regions);

        // append semantics flags
        if semantics.is_volatile {
            names.push("volatile");
        }
        if semantics.is_make_available {
            names.push("makeAvailable");
        }
        if semantics.is_make_visible {
            names.push("makeVisible");
        }

        // render as a single token or list
        if names.len() == 1 {
            names[0].to_string()
        } else {
            format!("[{}]", names.join(", "))
        }
    }

    fn collect_effect_region_names(&self, regions: MemoryRegionSet) -> Vec<&'static str> {
        // handle named region sets
        if regions == MemoryRegionSet::NONE {
            return vec!["none"];
        }
        if regions == MemoryRegionSet::ANY {
            return vec!["any"];
        }

        // collect ordered regions
        let ordered = [
            ("heap", MemoryRegionSet::HEAP),
            ("rawHeap", MemoryRegionSet::RAW_HEAP),
            ("stack", MemoryRegionSet::STACK),
            ("global", MemoryRegionSet::GLOBAL),
            ("shared", MemoryRegionSet::SHARED),
            ("local", MemoryRegionSet::LOCAL),
            ("constant", MemoryRegionSet::CONSTANT),
            ("io", MemoryRegionSet::IO),
        ];
        let mut names = Vec::new();
        for (name, set) in ordered {
            if regions.contains(set) {
                names.push(name);
            }
        }
        names
    }

    fn format_constant(&self, c: &Constant) -> String {
        match c {
            Constant::Null => "null".to_string(),
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
                    format!("{}float32", f32::from_bits(*bits as u32))
                } else {
                    format!("{}float64", f64::from_bits(*bits))
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
            CastOperator::FloatToSignedIntSaturating => "fptosi.sat",
            CastOperator::FloatToUnsignedIntSaturating => "fptoui.sat",
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
            Instruction::Error => {
                self.write_colored("<error instruction>", Color::BrightRed);
            }

            Instruction::Const { destination, value } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = ");
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
                self.write(" = local.address ");
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
                self.write(" = global.address ");
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
                self.write(" = function.address ");
                self.write(&self.format_function_id(*function));
            }
            Instruction::FunctionBind {
                destination,
                function,
                environment,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = function.bind ");
                self.write(&self.format_function_id(*function));
                self.write(", ");
                self.write(&self.format_value(*environment));
            }
            Instruction::FunctionEnvironment { destination } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = function.environment");
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

            Instruction::Dispose { value } => {
                self.write("dispose ");
                self.write(&self.format_value(*value));
            }

            Instruction::AsyncDispose { value } => {
                self.write("dispose.async ");
                self.write(&self.format_value(*value));
            }

            Instruction::Pin { value } => {
                self.write("pin ");
                self.write(&self.format_value(*value));
            }

            Instruction::Unpin { value } => {
                self.write("unpin ");
                self.write(&self.format_value(*value));
            }

            Instruction::Drop { value } => {
                self.write("drop ");
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
                self.write(" = field.address ");
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
                self.write(" = element.address ");
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
            Instruction::VectorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = vector.select ");
                self.write(&self.format_value(*mask));
                self.write(", ");
                self.write(&self.format_value(*then_value));
                self.write(", ");
                self.write(&self.format_value(*else_value));
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
                self.write(", axes(");
                for (i, axis) in axes.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&axis.to_string());
                }
                self.write(")");
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
                self.write(", dims(lhsBatch(");
                for (i, dim) in dimensions.lhs_batch.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), rhsBatch(");
                for (i, dim) in dimensions.rhs_batch.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), lhsContract(");
                for (i, dim) in dimensions.lhs_contracting.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), rhsContract(");
                for (i, dim) in dimensions.rhs_contracting.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("))");
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
                self.write(", dims(inputBatch(");
                self.write(&dimensions.input_batch.to_string());
                self.write("), inputFeature(");
                self.write(&dimensions.input_feature.to_string());
                self.write("), inputSpatial(");
                for (i, dim) in dimensions.input_spatial.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), kernelInputFeature(");
                self.write(&dimensions.kernel_input_feature.to_string());
                self.write("), kernelOutputFeature(");
                self.write(&dimensions.kernel_output_feature.to_string());
                self.write("), kernelSpatial(");
                for (i, dim) in dimensions.kernel_spatial.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), outputBatch(");
                self.write(&dimensions.output_batch.to_string());
                self.write("), outputFeature(");
                self.write(&dimensions.output_feature.to_string());
                self.write("), outputSpatial(");
                for (i, dim) in dimensions.output_spatial.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("))");
                self.write(", window(strides(");
                for (i, stride) in window.strides.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&stride.to_string());
                }
                self.write("), paddingLow(");
                for (i, pad) in window.padding_low.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&pad.to_string());
                }
                self.write("), paddingHigh(");
                for (i, pad) in window.padding_high.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&pad.to_string());
                }
                self.write("), lhsDilation(");
                for (i, dilation) in window.lhs_dilation.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dilation.to_string());
                }
                self.write("), rhsDilation(");
                for (i, dilation) in window.rhs_dilation.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dilation.to_string());
                }
                self.write("), windowReversal(");
                for (i, reverse) in window.window_reversal.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(if *reverse { "true" } else { "false" });
                }
                self.write(")), groups(feature(");
                self.write(&feature_group_count.to_string());
                self.write("), batch(");
                self.write(&batch_group_count.to_string());
                self.write("))");
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
                self.write(", dims(offsetDims(");
                for (i, dim) in dimensions.offset_dims.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), collapsedSliceDims(");
                for (i, dim) in dimensions.collapsed_slice_dims.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), startIndexMap(");
                for (i, dim) in dimensions.start_index_map.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), indexVectorDim(");
                self.write(&dimensions.index_vector_dim.to_string());
                self.write(")), sliceSizes(");
                for (i, size) in slice_sizes.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&size.to_string());
                }
                self.write(")");
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
                self.write(", dims(updateWindowDims(");
                for (i, dim) in dimensions.update_window_dims.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), insertedWindowDims(");
                for (i, dim) in dimensions.inserted_window_dims.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), scatterDimsToOperandDims(");
                for (i, dim) in dimensions.scatter_dims_to_operand_dims.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&dim.to_string());
                }
                self.write("), indexVectorDim(");
                self.write(&dimensions.index_vector_dim.to_string());
                self.write(")), mode(");
                self.write(mode.to_str());
                self.write(")");
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
            Instruction::TensorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = tensor.select ");
                self.write(&self.format_value(*mask));
                self.write(", ");
                self.write(&self.format_value(*then_value));
                self.write(", ");
                self.write(&self.format_value(*else_value));
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
                call,
                ..
            } => {
                if let Some(dst) = destination {
                    self.write_colored(&self.format_value(*dst), Color::Green);
                    self.write(" = ");
                }
                self.write("call ");
                self.write(&self.format_function_id(*function));
                self.write("(");
                let args = self.tree.get_arguments(call.arguments);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
            }

            Instruction::CallVirtual {
                destination,
                receiver,
                call,
                declaring_type,
                slot_id,
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
                self.write(&slot_id.0.to_string());
                self.write("(");
                let args = self.tree.get_arguments(call.arguments);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
            }

            Instruction::CallInterface {
                destination,
                receiver,
                call,
                declaring_type,
                slot_id,
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
                self.write(&slot_id.0.to_string());
                self.write("(");
                let args = self.tree.get_arguments(call.arguments);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
            }

            Instruction::CallIndirect {
                destination,
                callee,
                call,
                ..
            } => {
                if let Some(dst) = destination {
                    self.write_colored(&self.format_value(*dst), Color::Green);
                    self.write(" = ");
                }
                self.write("call.indirect ");
                self.write(&self.format_value(*callee));
                self.write("(");
                let args = self.tree.get_arguments(call.arguments);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
            }

            Instruction::New {
                destination,
                layout,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = new ");
                self.write_colored(&self.format_type_id(*layout), Color::Magenta);
                self.write(" -> ");
                self.write_colored(&self.format_type_id(*result_type), Color::Magenta);
            }

            Instruction::NewSlice {
                destination,
                element,
                length,
                result_type,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = new.slice ");
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

            Instruction::AtomicLoad {
                destination,
                pointer,
                ordering,
                scope,
                memory_scope,
                semantics,
                ..
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(" = atomic.load ");
                self.write(&self.format_value(*pointer));
                self.write(", ");
                self.write_colored(ordering.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(memory_scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(&self.format_memory_semantics(*semantics), Color::Yellow);
            }

            Instruction::AtomicStore {
                pointer,
                value,
                ordering,
                scope,
                memory_scope,
                semantics,
            } => {
                self.write("atomic.store ");
                self.write(&self.format_value(*pointer));
                self.write(", ");
                self.write(&self.format_value(*value));
                self.write(", ");
                self.write_colored(ordering.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(memory_scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(&self.format_memory_semantics(*semantics), Color::Yellow);
            }

            Instruction::AtomicCompareExchange {
                destination,
                pointer,
                expected,
                new_value,
                is_weak,
                ordering,
                scope,
                memory_scope,
                semantics,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                if *is_weak {
                    self.write(" = atomic.cas.weak ");
                } else {
                    self.write(" = atomic.cas ");
                }
                self.write(&self.format_value(*pointer));
                self.write(", ");
                self.write(&self.format_value(*expected));
                self.write(", ");
                self.write(&self.format_value(*new_value));
                self.write(", ");
                self.write_colored(ordering.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(memory_scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(&self.format_memory_semantics(*semantics), Color::Yellow);
            }

            Instruction::AtomicRmw {
                destination,
                operator,
                pointer,
                value,
                ordering,
                scope,
                memory_scope,
                semantics,
            } => {
                self.write_colored(&self.format_value(*destination), Color::Green);
                self.write(&format!(" = atomic.rmw.{operator} "));
                self.write(&self.format_value(*pointer));
                self.write(", ");
                self.write(&self.format_value(*value));
                self.write(", ");
                self.write_colored(ordering.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(memory_scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(&self.format_memory_semantics(*semantics), Color::Yellow);
            }

            Instruction::AtomicFence {
                ordering,
                scope,
                memory_scope,
                semantics,
            } => {
                self.write("atomic.fence ");
                self.write_colored(ordering.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(memory_scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(&self.format_memory_semantics(*semantics), Color::Yellow);
            }

            Instruction::Barrier {
                scope,
                memory_scope,
                semantics,
            } => {
                self.write("barrier ");
                self.write_colored(scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(memory_scope.to_str(), Color::Yellow);
                self.write(", ");
                self.write_colored(&self.format_memory_semantics(*semantics), Color::Yellow);
            }

            Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
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
                self.write(")");
            }
        }

        self.write("\n");
    }

    // === terminator dumping ===

    fn dump_terminator(&mut self, term: &Terminator) {
        self.write_indent();
        match term {
            Terminator::Error => {
                self.write_colored("<error terminator>", Color::BrightRed);
            }

            Terminator::Return { value } => {
                self.write_colored("return", Color::Red);
                if let Some(v) = value {
                    self.write(" ");
                    self.write(&self.format_value(*v));
                }
            }

            Terminator::Jump { target } => {
                self.write_colored("jump", Color::Red);
                self.write(" ");
                self.write_block_target(target);
            }

            Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                self.write_colored("branch", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*condition));
                self.write(", ");
                self.write_block_target(then_target);
                self.write(", ");
                self.write_block_target(else_target);
            }

            Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                self.write_colored("check", Color::Red);
                self.write(" ");
                match constraint {
                    CheckConstraint::Bounds {
                        index,
                        length,
                        collection,
                        is_signed,
                    } => {
                        let prefix = if *is_signed { "bounds.s" } else { "bounds.u" };
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
                        self.write("zeroDivisor ");
                        self.write(&self.format_value(*divisor));
                    }
                    CheckConstraint::Type { value, expected } => {
                        self.write("dynamicType ");
                        self.write(&self.format_value(*value));
                        self.write(", ");
                        self.write(&self.format_type_id(*expected));
                    }
                    CheckConstraint::Union { value, expected } => {
                        self.write("unionTag ");
                        self.write(&self.format_value(*value));
                        self.write(", ");
                        self.write(&expected.to_string());
                    }
                    CheckConstraint::ReceiverType { receiver, expected } => {
                        self.write("receiverType ");
                        self.write(&self.format_value(*receiver));
                        self.write(", ");
                        self.write(&self.format_type_id(*expected));
                    }
                    CheckConstraint::Implements { receiver, expected } => {
                        self.write("interfaceConformance ");
                        self.write(&self.format_value(*receiver));
                        self.write(", ");
                        self.write(&self.format_type_id(*expected));
                    }
                    CheckConstraint::ShiftRange {
                        value,
                        bit_width,
                        is_signed,
                    } => {
                        let prefix = if *is_signed {
                            "shiftRange.s"
                        } else {
                            "shiftRange.u"
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
                            "narrowRange.s"
                        } else {
                            "narrowRange.u"
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
                        let suffix = if *is_signed { "s" } else { "u" };
                        self.write(&format!(
                            "{}.overflow.{suffix}",
                            overflow_check_family(*operator)
                        ));
                        self.write(" ");
                        self.write(&self.format_value(*left));
                        self.write(", ");
                        self.write(&self.format_value(*right));
                    }
                }
                self.write(" -> ");
                self.write_block_target(success);
                self.write(", ");
                self.write_block_target(failure);
            }

            Terminator::Switch {
                value,
                default,
                cases,
            } => {
                self.write_colored("switch", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*value));
                self.write(", default ");
                self.write_block_target(default);
                for SwitchCase {
                    value: case_val,
                    target,
                } in cases
                {
                    let case_text = match case_val {
                        crate::IntegerReference::Integer(value) => value.to_string(),
                        crate::IntegerReference::Missing => "<missing>".to_string(),
                        crate::IntegerReference::Error => "<error>".to_string(),
                    };

                    self.write(&format!(", {case_text} => "));
                    self.write_block_target(target);
                }
            }

            Terminator::Unreachable => {
                self.write_colored("unreachable", Color::Red);
            }

            Terminator::Yield { value, resume } => {
                self.write_colored("yield", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*value));
                self.write(", ");
                self.write_block_target(resume);
            }

            Terminator::Invoke {
                function,
                call,
                normal_target,
                unwind_target,
            } => {
                self.write_colored("invoke", Color::Red);
                self.write(" ");
                self.write(&self.format_function_id(*function));
                self.write("(");
                for (i, arg) in call.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
                self.write(" -> ");
                self.write_block_target(normal_target);
                self.write(", catch ");
                self.write_block_target(unwind_target);
            }

            Terminator::InvokeIndirect {
                callee,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                self.write_colored("invoke.indirect", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*callee));
                self.write("(");
                for (i, arg) in call.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
                self.write(" -> ");
                self.write_block_target(normal_target);
                self.write(", catch ");
                self.write_block_target(unwind_target);
            }

            Terminator::InvokeVirtual {
                receiver,
                call,
                declaring_type,
                slot_id,
                normal_target,
                unwind_target,
                ..
            } => {
                self.write_colored("invoke.virtual", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*receiver));
                self.write(", ");
                self.write_colored(&self.format_type_id(*declaring_type), Color::Magenta);
                self.write(", ");
                self.write(&slot_id.0.to_string());
                self.write("(");
                for (i, arg) in call.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
                self.write(" -> ");
                self.write_block_target(normal_target);
                self.write(", catch ");
                self.write_block_target(unwind_target);
            }

            Terminator::InvokeInterface {
                receiver,
                call,
                declaring_type,
                slot_id,
                normal_target,
                unwind_target,
                ..
            } => {
                self.write_colored("invoke.interface", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*receiver));
                self.write(", ");
                self.write_colored(&self.format_type_id(*declaring_type), Color::Magenta);
                self.write(", ");
                self.write(&slot_id.0.to_string());
                self.write("(");
                for (i, arg) in call.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
                self.write(" -> ");
                self.write_block_target(normal_target);
                self.write(", catch ");
                self.write_block_target(unwind_target);
            }

            Terminator::Throw { value } => {
                self.write_colored("throw", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*value));
            }

            Terminator::Trap { kind, payload } => {
                self.write_colored(
                    match kind {
                        TrapKind::Abort => "trap.abort",
                        TrapKind::Panic => "trap.panic",
                    },
                    Color::Red,
                );

                if let Some(payload) = payload {
                    self.write(" ");
                    self.write(&self.format_value(*payload));
                }
            }

            Terminator::TailCall { function, call } => {
                self.write_colored("tailCall", Color::Red);
                self.write(" ");
                self.write(&self.format_function_id(*function));
                self.write("(");
                for (i, arg) in call.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
            }

            Terminator::TailCallIndirect { callee, call, .. } => {
                self.write_colored("tailCall.indirect", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*callee));
                self.write("(");
                for (i, arg) in call.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
            }

            Terminator::TailCallVirtual {
                receiver,
                call,
                declaring_type,
                slot_id,
                ..
            } => {
                self.write_colored("tailCall.virtual", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*receiver));
                self.write(", ");
                self.write_colored(&self.format_type_id(*declaring_type), Color::Magenta);
                self.write(", ");
                self.write(&slot_id.0.to_string());
                self.write("(");
                for (i, arg) in call.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
            }

            Terminator::TailCallInterface {
                receiver,
                call,
                declaring_type,
                slot_id,
                ..
            } => {
                self.write_colored("tailCall.interface", Color::Red);
                self.write(" ");
                self.write(&self.format_value(*receiver));
                self.write(", ");
                self.write_colored(&self.format_type_id(*declaring_type), Color::Magenta);
                self.write(", ");
                self.write(&slot_id.0.to_string());
                self.write("(");
                for (i, arg) in call.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.format_value(*arg));
                }
                self.write(")");
                self.write(" : ");
                self.write_colored(&self.format_type_id(call.signature), Color::Magenta);
            }
        }
        self.write("\n");
    }
}

/// Return the canonical operator family used in overflow checks.
fn overflow_check_family(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "int.add",
        BinaryOperator::Subtract => "int.sub",
        BinaryOperator::Multiply => "int.mul",
        BinaryOperator::SignedDivide | BinaryOperator::UnsignedDivide => "int.div",
        BinaryOperator::SignedRemainder | BinaryOperator::UnsignedRemainder => "int.rem",
        _ => panic!("unsupported overflow check operator: {operator:?}"),
    }
}

/// Split a packed tensor range list into offsets, sizes, and strides.
fn split_tensor_ranges(
    values: &[ValueReference],
    offsets_count: u16,
    sizes_count: u16,
    strides_count: u16,
) -> (&[ValueReference], &[ValueReference], &[ValueReference]) {
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
        if function.linkage.is_import() {
            self.write_colored("extern ", Color::BrightBlue);
        } else if function.linkage.is_exported() {
            self.write_colored("export ", Color::BrightBlue);
        }
        self.write_colored("function", Color::BrightBlue);
        self.write(" ");
        self.write(&self.format_function_id(id));
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
        self.write("): ");
        self.write_colored(&self.format_type_id(function.return_type), Color::Magenta);
        self.write(" {\n");

        self.indent();

        // locals
        if !function.locals.is_empty() {
            for local_id in &function.locals {
                let local = tree.get(*local_id);
                self.write_indent();
                self.write("local ");
                self.write(&self.format_local_id(*local_id));
                self.write(": ");
                self.write_colored(&self.format_type_id(local.ty), Color::Magenta);
                self.write(", ");
                match local.ownership {
                    Ownership::Owned => self.write("owned"),
                    Ownership::Borrowed => self.write("borrowed"),
                    Ownership::Copy => self.write("copy"),
                }
                if local.mutability == Mutability::Immutable {
                    self.write(", readonly");
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

        let terminator = tree.get(block.terminator);

        // instructions
        for inst_id in &block.instructions {
            let instruction = tree.get(*inst_id);
            self.dump_instruction(instruction);
        }

        // terminator
        self.dump_terminator(terminator);

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
        self.write(" ");
        self.write(&self.format_global_id(id));
        self.write(": ");
        self.write_colored(&self.format_type_id(global.ty), Color::Magenta);
        if global.mutability == Mutability::Immutable {
            self.write(", readonly");
        }
        if let Some(init) = &global.initializer {
            self.write(" = ");
            self.dump_data_init(init);
        }
        self.write("\n");
    }
}

impl<'a> Dumper<'a> {
    /// Dump a data initializer.
    fn dump_data_init(&mut self, init: &GlobalInitializer) {
        match init {
            GlobalInitializer::Zero => self.write("zeroInit"),
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

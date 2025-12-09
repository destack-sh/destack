//! Module-level lowering from MIR to Cranelift IR.
//!
//! Module lowering works in two phases:
//!
//! 1. **Declaration phase**: Declare all functions with their signatures.
//!    This allows functions to call each other regardless of definition order.
//!
//! 2. **Definition phase**: Lower each function body using `FunctionLowerer`.
//!    The function id map from phase 1 is used to resolve call targets.

use std::collections::HashMap;
use std::sync::Arc;

use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::{Context, ir as cir};
use cranelift_module::{DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use destack_mir as mir;
use destack_source::StringPool;

use super::FunctionLowerer;
use super::layout::compute_type_layout;
use super::r#type::lower_type;
use crate::{CodegenCraneliftError, CodegenCraneliftResult, CodegenCraneliftWarning};

/// Internal output from module lowering (bytes + warnings).
#[derive(Debug)]
pub(crate) struct ModuleLowerOutput {
    /// Generated binary output (object file or wasm).
    pub bytes: Vec<u8>,
    /// Warnings encountered during generation.
    pub warnings: Vec<CodegenCraneliftWarning>,
    /// Non-fatal errors encountered during generation.
    pub errors: Vec<CodegenCraneliftError>,
}

/// Module context for lowering a MIR module to Cranelift.
pub(crate) struct ModuleLowerer<'a> {
    /// The target ISA.
    isa: Arc<dyn TargetIsa>,
    /// String pool for interned names.
    strings: &'a StringPool,
    /// The Cranelift object module.
    cl_module: ObjectModule,
    /// Mapping from MIR function ids to Cranelift function ids.
    cl_function_ids: HashMap<mir::LocalNodeId<mir::Function>, FuncId>,
    /// Mapping from MIR global ids to Cranelift data ids.
    cl_global_data_ids: HashMap<mir::LocalNodeId<mir::Global>, DataId>,
    /// Compiled Cranelift functions (for CLIF output).
    cl_functions: Vec<(String, cir::Function)>,
    /// Collected warnings.
    warnings: Vec<CodegenCraneliftWarning>,
    /// Collected non-fatal errors (treated as warnings for continued processing).
    errors: Vec<CodegenCraneliftError>,
}

impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(isa: Arc<dyn TargetIsa>, strings: &'a StringPool, name: &str) -> Self {
        let builder =
            ObjectBuilder::new(isa.clone(), name, cranelift_module::default_libcall_names())
                .expect("failed to create object builder");
        let module = ObjectModule::new(builder);
        Self {
            isa,
            strings,
            cl_module: module,
            cl_function_ids: HashMap::new(),
            cl_global_data_ids: HashMap::new(),
            cl_functions: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Record a warning.
    #[allow(dead_code)]
    pub(crate) fn warning(&mut self, warning: CodegenCraneliftWarning) {
        self.warnings.push(warning);
    }

    /// Lower an entire MIR module.
    pub(crate) fn lower_module(&mut self, tree: &mir::NodeTree) -> CodegenCraneliftResult<()> {
        // phase 1: declare and define all globals
        self.lower_globals(tree)?;

        // phase 2: declare all functions
        self.declare_functions(tree)?;

        // phase 3: lower function bodies
        self.lower_functions(tree)?;

        Ok(())
    }

    /// Declare all functions in the module (first pass).
    fn declare_functions(&mut self, tree: &mir::NodeTree) -> CodegenCraneliftResult<()> {
        let pointer_bytes = self.isa.pointer_bytes();

        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            let signature = self.create_signature(tree, function, pointer_bytes)?;
            let name = self.strings.get(function.name);

            // create cranelift function
            let cl_function_id =
                self.cl_module
                    .declare_function(&name, Linkage::Export, &signature)?;
            self.cl_function_ids.insert(function_id, cl_function_id);
        }

        Ok(())
    }

    /// Lower all globals to Cranelift data sections.
    fn lower_globals(&mut self, tree: &mir::NodeTree) -> CodegenCraneliftResult<()> {
        let pointer_bytes = self.isa.pointer_bytes();

        for (global_id, global) in tree.iter_nodes::<mir::Global>() {
            let name = self.strings.get(global.name);

            // determine linkage: private names (starting with . or _) are local
            let linkage = if name.starts_with('.') || name.starts_with('_') {
                Linkage::Local
            } else {
                Linkage::Export
            };

            // declare the data section
            let writable = global.is_mutable();
            let data_id = self
                .cl_module
                .declare_data(&name, linkage, writable, false)?;

            // define the data if we have an initializer
            if let Some(ref init) = global.initializer {
                let bytes = self.lower_initializer(tree, init, global.ty, pointer_bytes)?;
                let mut data_description = cranelift_module::DataDescription::new();
                data_description.define(bytes.into_boxed_slice());
                self.cl_module.define_data(data_id, &data_description)?;
            }

            self.cl_global_data_ids.insert(global_id, data_id);
        }

        Ok(())
    }

    /// Lower a GlobalInitializer to raw bytes.
    fn lower_initializer(
        &self,
        tree: &mir::NodeTree,
        init: &mir::GlobalInitializer,
        ty: mir::LocalNodeId<mir::Type>,
        pointer_bytes: u8,
    ) -> CodegenCraneliftResult<Vec<u8>> {
        match init {
            mir::GlobalInitializer::Zero => {
                // compute size from type and return zero bytes
                let layout = compute_type_layout(tree, ty, pointer_bytes)?;
                Ok(vec![0u8; layout.size as usize])
            }

            mir::GlobalInitializer::Scalar(constant) => self.lower_scalar_constant(constant, ty),

            mir::GlobalInitializer::Bytes(bytes) => Ok(bytes.clone()),

            mir::GlobalInitializer::Aggregate(elements) => {
                // get the element types from the type
                let mir_type = tree.get(ty);
                let element_types = match mir_type {
                    mir::Type::Tuple { elements } => elements.clone(),
                    mir::Type::Struct { fields } => {
                        fields.iter().map(|f| tree.get(*f).ty).collect()
                    }
                    mir::Type::Array { element, length } => {
                        vec![*element; *length as usize]
                    }
                    _ => {
                        return Err(CodegenCraneliftError::unsupported_type(
                            format!("aggregate initializer for non-aggregate type: {mir_type:?}"),
                            ty.into_any(),
                        ));
                    }
                };

                // lower each element and concatenate
                // NOTE #Incomplete: this doesn't handle alignment padding between fields
                let mut bytes = Vec::new();
                for (elem_init, elem_ty) in elements.iter().zip(element_types.iter()) {
                    let elem_bytes =
                        self.lower_initializer(tree, elem_init, *elem_ty, pointer_bytes)?;
                    bytes.extend(elem_bytes);
                }
                Ok(bytes)
            }
        }
    }

    /// Lower a scalar constant to bytes.
    fn lower_scalar_constant(
        &self,
        constant: &mir::Constant,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CodegenCraneliftResult<Vec<u8>> {
        match constant {
            // boolean -> 1 or 0
            mir::Constant::Boolean { value } => Ok(vec![if *value { 1 } else { 0 }]),

            // integer -> bytes
            mir::Constant::Int { width, .. } | mir::Constant::UInt { width, .. } => {
                let value = match constant {
                    mir::Constant::Int { value, .. } => *value as u64,
                    mir::Constant::UInt { value, .. } => *value,
                    _ => unreachable!(),
                };
                let bytes = match width {
                    8 => vec![value as u8],
                    16 => (value as u16).to_le_bytes().to_vec(),
                    32 => (value as u32).to_le_bytes().to_vec(),
                    64 => value.to_le_bytes().to_vec(),
                    128 => (value as u128).to_le_bytes().to_vec(),
                    _ => {
                        return Err(CodegenCraneliftError::unsupported_type(
                            format!("unsupported integer width for global initializer: {width}"),
                            ty.into_any(),
                        ));
                    }
                };
                Ok(bytes)
            }

            // float -> bytes
            mir::Constant::Float { bits, width } => {
                let bytes = match width {
                    32 => (*bits as u32).to_le_bytes().to_vec(),
                    64 => bits.to_le_bytes().to_vec(),
                    _ => {
                        return Err(CodegenCraneliftError::unsupported_type(
                            format!("unsupported float width for global initializer: {width}"),
                            ty.into_any(),
                        ));
                    }
                };
                Ok(bytes)
            }

            // char -> bytes
            mir::Constant::Char { value } => {
                // char is stored as u32 (unicode codepoint)
                Ok((*value as u32).to_le_bytes().to_vec())
            }

            // string -> error (should use GlobalInitializer::Bytes instead)
            mir::Constant::String { .. } => Err(CodegenCraneliftError::unsupported_type(
                "string constants in scalar position; use GlobalInitializer::Bytes".to_string(),
                ty.into_any(),
            )),
        }
    }

    /// Lower / define all function bodies (third pass after declaration).
    fn lower_functions(&mut self, tree: &mir::NodeTree) -> CodegenCraneliftResult<()> {
        let pointer_bytes = self.isa.pointer_bytes();

        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            let cl_function_id = self.cl_function_ids[&function_id];
            let name = self.strings.get(function.name).to_string();

            // create cranelift context
            let mut context = Context::new();
            context.func.signature = self.create_signature(tree, function, pointer_bytes)?;
            context.func.name = cir::UserFuncName::user(0, cl_function_id.as_u32());

            // lower the function body
            let function_lowerer = FunctionLowerer::new(
                tree,
                self.strings,
                function,
                &self.isa,
                &mut self.cl_module,
                &self.cl_function_ids,
                &self.cl_global_data_ids,
                pointer_bytes,
            );
            function_lowerer.lower(&mut context.func)?;

            // save for CLIF output
            self.cl_functions.push((name, context.func.clone()));

            // compile and define
            self.cl_module
                .define_function(cl_function_id, &mut context)?;
        }

        Ok(())
    }

    /// Create a Cranelift signature for a MIR function.
    fn create_signature(
        &self,
        tree: &mir::NodeTree,
        function: &mir::Function,
        pointer_bytes: u8,
    ) -> Result<cir::Signature, CodegenCraneliftError> {
        let call_conv = self.isa.default_call_conv();
        let mut signature = cir::Signature::new(call_conv);

        // parameters
        for param in &function.parameters {
            let ty = lower_type(tree, param.ty, pointer_bytes)?;
            signature.params.push(cir::AbiParam::new(ty));
        }

        // return type (if not void)
        if !matches!(tree.get(function.return_type), mir::Type::Void) {
            let ty = lower_type(tree, function.return_type, pointer_bytes)?;
            signature.returns.push(cir::AbiParam::new(ty));
        }

        Ok(signature)
    }

    /// Finish lowering and produce output.
    pub(crate) fn finish(self) -> Result<ModuleLowerOutput, CodegenCraneliftError> {
        let product = self.cl_module.finish();
        let bytes = product
            .emit()
            .map_err(|e| CodegenCraneliftError::Internal {
                message: e.to_string(),
            })?;

        Ok(ModuleLowerOutput {
            bytes,
            warnings: self.warnings,
            errors: self.errors,
        })
    }

    /// Get the Cranelift IR text format for all functions.
    pub(crate) fn as_clif_string(&self) -> CodegenCraneliftResult<String> {
        let mut output = String::new();
        for (name, func) in &self.cl_functions {
            output.push_str(&format!("; function: {name}\n"));
            output.push_str(&func.display().to_string());
            output.push('\n');
        }
        Ok(output)
    }
}

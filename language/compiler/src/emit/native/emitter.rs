use std::fmt;
use std::mem::offset_of;
use std::str::FromStr;
use std::sync::Arc;

use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{Context, ir as cir};
use cranelift_module::{DataId, FuncId, Linkage, Module, ModuleReloc, ModuleRelocTarget};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::{Endianness, Triple};
use tspp_artifact::MirOptimized;
use tspp_core::{FxIndexMap, FxIndexSet};
use tspp_mir as mir;
use tspp_native as native;
use tspp_repository::{OptimizeLevel, Target};
use tspp_source::ModuleId;

use crate::{EmitError, ObjectEmitter};

use super::function::{FunctionEmitter, StackMap};
use super::object::{SymbolTable, UnwindEmitter};
use super::r#type::TypeEmitter;

/// Emit one target-dependent relocatable native object.
pub struct NativeEmitter<'a> {
    /// Module receiving diagnostics.
    module: ModuleId,
    /// Optimized MIR being emitted.
    optimized: &'a MirOptimized,
    /// Object-local identities and logical frame states.
    object: &'a ObjectEmitter,
    /// Native target ISA.
    isa: Arc<dyn TargetIsa>,
    /// Cranelift declaration and compilation module.
    output: ObjectModule,
    /// Object-local native symbols.
    symbols: SymbolTable,
    /// Target-native unwind tables.
    unwind: UnwindEmitter,
    /// Internal native functions keyed by MIR identity.
    functions: FxIndexMap<mir::FunctionId, FuncId>,
    /// Canonical entries keyed by MIR identity.
    entries: FxIndexMap<mir::FunctionId, FuncId>,
    /// Object-local function indices keyed by Cranelift identity.
    function_indices: FxIndexMap<FuncId, u32>,
    /// Platform imports keyed by Cranelift identity.
    imports: FxIndexMap<FuncId, native::Import>,
    /// Native definitions in object-local function order.
    definitions: Vec<Option<native::DefinitionBuilder>>,
    /// Independently placed code blocks in object-local order.
    blocks: Vec<Option<native::BlockBuilder>>,
    /// Object-local native trap sites.
    traps: Vec<native::ObjectTrap>,
    /// Physical native frame maps.
    frame_maps: Vec<native::ObjectFrameMapBuilder>,
    /// Explicit target CPU features.
    features: Vec<String>,
}

impl fmt::Debug for NativeEmitter<'_> {
    /// Format the native emitter without exposing Cranelift state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeEmitter")
            .field("module", &self.module)
            .field("target", &self.isa.triple())
            .field("functions", &self.functions.len())
            .field("blocks", &self.blocks.len())
            .field("traps", &self.traps.len())
            .field("frame_maps", &self.frame_maps.len())
            .finish()
    }
}

impl<'a> NativeEmitter<'a> {
    /// Create one native emitter for a target.
    pub fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        object: &'a ObjectEmitter,
        target: &Target,
    ) -> Result<Self, EmitError> {
        // select the instruction set and check the MIR target layout
        let triple = Self::triple(module, target)?;
        let isa = Self::isa(module, target, triple)?;
        Self::require_host_abi(module, optimized.target, isa.as_ref())?;
        let features = Self::features(isa.as_ref());

        // create the native object module
        let builder = ObjectBuilder::new(
            isa.clone(),
            format!("tspp_{module}"),
            cranelift_module::default_libcall_names(),
        )
        .map_err(|error| Self::internal(module, error.to_string()))?;
        let output = ObjectModule::new(builder);

        // create the target's unwind emitter
        let unwind = UnwindEmitter::new(module, isa.as_ref())?;

        Ok(Self {
            module,
            optimized,
            object,
            isa,
            output,
            symbols: SymbolTable::default(),
            unwind,
            functions: FxIndexMap::default(),
            entries: FxIndexMap::default(),
            function_indices: FxIndexMap::default(),
            imports: FxIndexMap::default(),
            definitions: Vec::new(),
            blocks: Vec::new(),
            traps: Vec::new(),
            frame_maps: Vec::new(),
            features,
        })
    }

    /// Emit the complete relocatable native object.
    pub fn emit(mut self) -> Result<native::Object, EmitError> {
        self.declare_functions()?;
        self.emit_functions()?;

        // require every reserved code block to have been compiled
        let blocks = self
            .blocks
            .into_iter()
            .enumerate()
            .map(|(index, block)| {
                block.ok_or_else(|| {
                    Self::internal(self.module, format!("native block {index} was not emitted"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let target = self.isa.triple().to_string();
        let unwind = self.unwind.build()?;

        Ok(native::ObjectBuilder::new(target, self.optimized.target)
            .features(self.features)
            .symbols(self.symbols.into_values())
            .blocks(blocks)
            .definitions(self.definitions)
            .unwind(unwind)
            .map(
                native::ObjectMapBuilder::new()
                    .traps(self.traps)
                    .frames(self.frame_maps),
            )
            .build())
    }

    /// Declare every typed body and canonical entry.
    fn declare_functions(&mut self) -> Result<(), EmitError> {
        // create the type emitter for function signatures
        let isa = self.isa.clone();
        let types = TypeEmitter::new(
            self.module,
            self.optimized.target,
            self.optimized,
            isa.as_ref(),
        );

        // reserve stable function, definition, and block identities
        for index in 0..self.object.functions().len() {
            let id = self.object.functions()[index];
            let function = self.optimized.tree.get(id);
            let signature = types.signature(function)?;
            let linkage = Self::linkage(function.linkage);
            let function_id = self
                .output
                .declare_function(&Self::body_symbol(function.symbol), linkage, &signature)
                .map_err(|error| Self::internal(self.module, error.to_string()))?;
            self.functions.insert(id, function_id);
            self.function_indices.insert(function_id, index as u32);

            // record an imported function
            if !function.is_defined() {
                self.definitions.push(None);
                continue;
            }

            let entry_id = self
                .output
                .declare_function(
                    &Self::entry_symbol(function.symbol),
                    Linkage::Export,
                    &types.entry_signature(),
                )
                .map_err(|error| Self::internal(self.module, error.to_string()))?;
            self.entries.insert(id, entry_id);
            let body = self.reserve_block();
            let entry = self.reserve_block();
            self.definitions
                .push(Some(native::DefinitionBuilder::new(body, entry)));
        }

        Ok(())
    }

    /// Emit every defined body and canonical entry.
    fn emit_functions(&mut self) -> Result<(), EmitError> {
        // compile each defined function
        for index in 0..self.object.functions().len() {
            let id = self.object.functions()[index];
            let function = self.optimized.tree.get(id);
            let Some(definition) = self.definitions[index].as_ref() else {
                continue;
            };
            let body = definition.body();
            let entry = definition.entry();
            let frame_base = self.frame_maps.len() as u32;
            let (mut context, stack_maps) = self.lower_body(index, id, function, frame_base)?;
            let function_id = self.functions[&id];

            // compile the typed native body
            self.output
                .define_function(function_id, &mut context)
                .map_err(|error| Self::internal(self.module, error.to_string()))?;
            let frame_maps = Self::frame_maps(self.module, body, &context, stack_maps)?;
            self.frame_maps.extend(frame_maps);
            self.blocks[body.index()] = Some(self.block(function_id, body, &context)?);

            // compile the canonical engine-transition entry
            let entry_id = self.entries[&id];
            let mut context = self.lower_entry(index, function_id, function)?;
            self.output
                .define_function(entry_id, &mut context)
                .map_err(|error| Self::internal(self.module, error.to_string()))?;
            self.blocks[entry.index()] = Some(self.block(entry_id, entry, &context)?);
        }

        Ok(())
    }

    /// Lower one typed native body into Cranelift IR.
    fn lower_body(
        &mut self,
        index: usize,
        id: mir::FunctionId,
        function: &mir::Function,
        frame_base: u32,
    ) -> Result<(Context, Vec<StackMap>), EmitError> {
        // read the declared function identity and signature
        let function_id = self.functions[&id];
        let isa = self.isa.clone();
        let types = TypeEmitter::new(
            self.module,
            self.optimized.target,
            self.optimized,
            isa.as_ref(),
        );

        // initialize the Cranelift function signature
        let mut context = Context::new();
        context.func.signature = types.signature(function)?;
        context.func.name = cir::UserFuncName::user(0, function_id.as_u32());

        // check the activation stack limit in the prologue
        let activation = context
            .func
            .create_global_value(cir::GlobalValueData::VMContext);
        let stack_limit_offset = offset_of!(native::abi::Activation, stack_limit) as i32;
        let flags = context
            .func
            .dfg
            .mem_flags
            .insert(cir::MemFlagsData::trusted())
            .map_err(|_| Self::internal(self.module, "native stack limit flags overflow"))?;
        let stack_limit = context
            .func
            .create_global_value(cir::GlobalValueData::Load {
                base: activation,
                offset: stack_limit_offset.into(),
                global_type: isa.pointer_type(),
                flags,
            });
        context.func.stack_limit = Some(stack_limit);

        // lower the body and collect its stack maps
        let stack_maps = FunctionEmitter::new(
            self.module,
            self.optimized,
            self.object,
            &types,
            &mut self.output,
            &mut self.symbols,
            &self.functions,
            &mut self.imports,
            id,
            function,
            index as u32,
            frame_base,
        )?
        .emit(&mut context.func)?;

        // verify the generated Cranelift function
        self.verify(&context)?;

        Ok((context, stack_maps))
    }

    /// Lower one canonical engine-transition entry into Cranelift IR.
    fn lower_entry(
        &mut self,
        index: usize,
        function_id: FuncId,
        function: &mir::Function,
    ) -> Result<Context, EmitError> {
        // create the type emitter for the entry signature
        let isa = self.isa.clone();
        let types = TypeEmitter::new(
            self.module,
            self.optimized.target,
            self.optimized,
            isa.as_ref(),
        );
        let mut context = Context::new();
        context.func.signature = types.entry_signature();
        context.func.name = cir::UserFuncName::user(1, index as u32);
        FunctionEmitter::emit_entry(
            self.module,
            &types,
            &mut self.output,
            function_id,
            function,
            &mut context.func,
        )?;

        // verify the generated Cranelift function
        self.verify(&context)?;

        Ok(context)
    }

    /// Format canonical Cranelift IR emitted for tests.
    #[cfg(test)]
    pub(crate) fn format_cranelift(mut self) -> Result<String, EmitError> {
        self.declare_functions()?;
        let mut rendered = Vec::new();
        let mut frame_base = 0;

        // lower every body and entry through the production path
        for index in 0..self.object.functions().len() {
            let id = self.object.functions()[index];
            let function = self.optimized.tree.get(id);
            if !function.is_defined() {
                continue;
            }
            let (context, stack_maps) = self.lower_body(index, id, function, frame_base)?;
            rendered.push(context.func.display().to_string());
            frame_base += stack_maps.len() as u32;
            let function_id = self.functions[&id];
            let context = self.lower_entry(index, function_id, function)?;
            rendered.push(context.func.display().to_string());
        }

        // normalize the host calling convention for portable snapshots
        let call_conv = self.isa.default_call_conv().to_string();

        Ok(rendered.join("\n").replace(&call_conv, "native"))
    }

    /// Build one object code block from compiled Cranelift code.
    fn block(
        &mut self,
        function: FuncId,
        block: native::BlockId,
        context: &Context,
    ) -> Result<native::BlockBuilder, EmitError> {
        let compiled = context
            .compiled_code()
            .ok_or_else(|| Self::internal(self.module, "native function was not compiled"))?;
        let alignment = native::Alignment::new(compiled.buffer.alignment)
            .ok_or_else(|| Self::internal(self.module, "native function has invalid alignment"))?;

        // convert every Cranelift relocation into an object-local symbol reference
        let relocations = compiled
            .buffer
            .relocs()
            .iter()
            .map(|relocation| ModuleReloc::from_mach_reloc(relocation, &context.func, function))
            .collect::<Vec<_>>();
        let relocations = relocations
            .into_iter()
            .map(|relocation| self.relocation(block, relocation))
            .collect::<Result<Vec<_>, _>>()?;

        // retain every machine trap site in language terms
        let traps = compiled
            .buffer
            .traps()
            .iter()
            .map(|record| {
                let trap = Self::classify_trap(self.module, record.code)?;

                Ok(native::ObjectTrap::new(block, record.offset, trap))
            })
            .collect::<Result<Vec<_>, EmitError>>()?;
        self.traps.extend(traps);

        // retain standard frame and exception records for this code block
        let symbol = self
            .symbols
            .insert(native::Symbol::Block { block, offset: 0 });
        self.unwind.add(symbol, compiled, self.isa.as_ref())?;

        Ok(native::BlockBuilder::new(compiled.buffer.data(), alignment).relocations(relocations))
    }

    /// Convert one Cranelift trap into its language ABI classification.
    fn classify_trap(
        module: ModuleId,
        code: cir::TrapCode,
    ) -> Result<native::abi::Trap, EmitError> {
        let trap = match code {
            code if code == cir::TrapCode::STACK_OVERFLOW => native::abi::Trap::StackOverflow,
            code if code == cir::TrapCode::HEAP_OUT_OF_BOUNDS => native::abi::Trap::Bounds,
            code if code == cir::TrapCode::INTEGER_OVERFLOW
                || code == cir::TrapCode::BAD_CONVERSION_TO_INTEGER =>
            {
                native::abi::Trap::IntegerOverflow
            }
            code if code == cir::TrapCode::INTEGER_DIVISION_BY_ZERO => {
                native::abi::Trap::DivisionByZero
            }
            code => {
                let code = u32::from(code.as_raw().get());

                native::abi::Trap::try_from(code).map_err(|_| {
                    Self::internal(
                        module,
                        format!("native function uses unknown trap code {code}"),
                    )
                })?
            }
        };

        Ok(trap)
    }

    /// Convert one Cranelift relocation into an object-local relocation.
    fn relocation(
        &mut self,
        block: native::BlockId,
        relocation: ModuleReloc,
    ) -> Result<native::Relocation, EmitError> {
        let target = match relocation.name {
            ModuleRelocTarget::User {
                namespace: 0,
                index,
            } => {
                let function = FuncId::from_u32(index);
                if let Some(function) = self.function_indices.get(&function).copied() {
                    self.symbols.insert(native::Symbol::Function { function })
                } else if let Some(import) = self.imports.get(&function).copied() {
                    self.symbols.insert(native::Symbol::Import(import))
                } else {
                    return Err(Self::internal(
                        self.module,
                        "native relocation references an unknown function",
                    ));
                }
            }
            ModuleRelocTarget::User {
                namespace: 1,
                index,
            } => {
                let data = DataId::from_u32(index);

                self.symbols.data(data).ok_or_else(|| {
                    Self::internal(
                        self.module,
                        "native relocation references an unknown data symbol",
                    )
                })?
            }
            ModuleRelocTarget::User { .. } => {
                return Err(Self::internal(
                    self.module,
                    "native relocation references an unknown object symbol",
                ));
            }
            ModuleRelocTarget::FunctionOffset(_, offset) => {
                self.symbols.insert(native::Symbol::Block { block, offset })
            }
            ModuleRelocTarget::LibCall(libcall) => {
                let import = Self::libcall(self.module, libcall)?;

                self.symbols.insert(native::Symbol::Import(import))
            }
            ModuleRelocTarget::KnownSymbol(symbol) => {
                return Err(Self::internal(
                    self.module,
                    format!("unsupported native linker symbol {symbol}"),
                ));
            }
        };
        let kind = Self::relocation_kind(self.module, relocation.kind)?;

        Ok(native::Relocation::new(
            relocation.offset,
            target,
            relocation.addend,
            kind,
        ))
    }

    /// Project Cranelift stack maps into exact native frame maps.
    fn frame_maps(
        module: ModuleId,
        block: native::BlockId,
        context: &Context,
        stack_maps: Vec<StackMap>,
    ) -> Result<Vec<native::ObjectFrameMapBuilder>, EmitError> {
        let compiled = context
            .compiled_code()
            .ok_or_else(|| Self::internal(module, "native function was not compiled"))?;
        let maps = compiled.buffer.user_stack_maps();
        let layout = compiled
            .buffer
            .frame_layout()
            .ok_or_else(|| Self::internal(module, "native frame layout is missing"))?;
        let locations = layout
            .stackslots
            .values()
            .filter_map(|slot| slot.key.map(|key| (key, slot.offset)))
            .collect::<FxIndexMap<_, _>>();
        let mut frames = Vec::with_capacity(stack_maps.len());

        // resolve every requested stack map through its keyed marker slot
        for stack_map in stack_maps {
            let marker = locations
                .get(&stack_map.marker)
                .copied()
                .ok_or_else(|| Self::internal(module, "native frame marker is missing"))?;
            let (return_offset, map) = maps
                .iter()
                .find_map(|(offset, _, map)| {
                    map.entries()
                        .any(|(_, offset)| offset == marker)
                        .then_some((*offset, map))
                })
                .ok_or_else(|| {
                    Self::internal(
                        module,
                        format!("native stack map for frame marker {marker} is missing"),
                    )
                })?;
            let frame_pointer_offset = i64::from(layout.frame_to_fp_offset) - i64::from(marker);
            let frame_pointer_offset = i32::try_from(frame_pointer_offset)
                .map_err(|_| Self::internal(module, "native frame pointer offset exceeds i32"))?;
            let values = Self::frame_values(module, map, &locations, marker, &stack_map)?;
            frames.push(
                native::ObjectFrameMapBuilder::new(
                    block,
                    return_offset,
                    frame_pointer_offset,
                    stack_map.state,
                )
                .values(values),
            );
        }

        Ok(frames)
    }

    /// Build canonical frame values from one compiled stack map.
    fn frame_values(
        module: ModuleId,
        map: &cir::UserStackMap,
        locations: &FxIndexMap<cir::StackSlotKey, u32>,
        marker: u32,
        stack_map: &StackMap,
    ) -> Result<Vec<native::FrameValueBuilder>, EmitError> {
        let live = map
            .entries()
            .map(|(_, offset)| offset)
            .collect::<FxIndexSet<_>>();
        let values = stack_map
            .values
            .iter()
            .map(|(key, byte_len)| {
                let offset = locations.get(key).copied().ok_or_else(|| {
                    Self::internal(module, "native frame value location is missing")
                })?;
                if !live.contains(&offset) {
                    return Err(Self::internal(
                        module,
                        "native frame value is absent from its stack map",
                    ));
                }
                let offset = i64::from(offset) - i64::from(marker);
                let offset = i32::try_from(offset)
                    .map_err(|_| Self::internal(module, "native frame offset exceeds i32"))?;
                let location =
                    native::FrameLocation::new(native::FrameSource::Stack, offset, 0, *byte_len);

                Ok(native::FrameValueBuilder::new().locations([location]))
            })
            .collect::<Result<Vec<_>, EmitError>>()?;

        Ok(values)
    }

    /// Verify one lowered Cranelift function.
    fn verify(&self, context: &Context) -> Result<(), EmitError> {
        if let Err(errors) = cranelift_codegen::verify_function(&context.func, self.isa.as_ref()) {
            return Err(Self::internal(
                self.module,
                format!("{errors}\n{}", context.func.display()),
            ));
        }

        Ok(())
    }

    /// Reserve one stable object-local code block.
    fn reserve_block(&mut self) -> native::BlockId {
        let block = native::BlockId(self.blocks.len() as u32);
        self.blocks.push(None);

        block
    }

    /// Resolve the target triple.
    fn triple(module: ModuleId, target: &Target) -> Result<Triple, EmitError> {
        if let Some(triple) = target.resolved_target_triple() {
            return Triple::from_str(&triple)
                .map_err(|error| Self::internal(module, error.to_string()));
        }

        cranelift_native::builder()
            .map(|builder| builder.triple().clone())
            .map_err(|error| Self::internal(module, error.to_string()))
    }

    /// Build the Cranelift target ISA.
    fn isa(
        module: ModuleId,
        target: &Target,
        triple: Triple,
    ) -> Result<Arc<dyn TargetIsa>, EmitError> {
        let mut flags = settings::builder();
        let optimize = match target.compiler.optimize {
            OptimizeLevel::O0 => "none",
            OptimizeLevel::O1 | OptimizeLevel::O2 | OptimizeLevel::O3 | OptimizeLevel::O4 => {
                "speed"
            }
        };
        flags
            .set("opt_level", optimize)
            .map_err(|error| Self::internal(module, error.to_string()))?;
        // use direct relative references because every Program symbol is colocated
        flags
            .set("is_pic", "false")
            .map_err(|error| Self::internal(module, error.to_string()))?;
        flags
            .set("use_colocated_libcalls", "true")
            .map_err(|error| Self::internal(module, error.to_string()))?;
        flags
            .set("preserve_frame_pointers", "true")
            .map_err(|error| Self::internal(module, error.to_string()))?;
        flags
            .set("unwind_info", "true")
            .map_err(|error| Self::internal(module, error.to_string()))?;
        flags
            .set("enable_probestack", "true")
            .map_err(|error| Self::internal(module, error.to_string()))?;
        flags
            .set("probestack_strategy", "inline")
            .map_err(|error| Self::internal(module, error.to_string()))?;

        let mut builder = cranelift_native::builder()
            .map_err(|error| Self::internal(module, error.to_string()))?;
        if builder.triple() != &triple {
            return Err(Self::internal(
                module,
                format!(
                    "native runtime ABI cannot emit target {triple} from host {}",
                    builder.triple()
                ),
            ));
        }

        if matches!(
            triple.architecture,
            target_lexicon::Architecture::Aarch64(_)
        ) && triple.operating_system.is_like_darwin()
        {
            builder
                .set("sign_return_address", "false")
                .map_err(|error| Self::internal(module, error.to_string()))?;
            builder
                .set("sign_return_address_with_bkey", "false")
                .map_err(|error| Self::internal(module, error.to_string()))?;
        }

        // apply explicit target features after the required host ABI features
        if let Some(cpu) = &target.native.cpu {
            builder
                .enable(cpu)
                .map_err(|error| Self::internal(module, error.to_string()))?;
        }
        for feature in &target.native.cpu_features {
            builder
                .enable(feature)
                .map_err(|error| Self::internal(module, error.to_string()))?;
        }
        builder
            .finish(settings::Flags::new(flags))
            .map_err(|error| Self::internal(module, error.to_string()))
    }

    /// Require the process-local Rust ABI used by the native runtime table.
    fn require_host_abi(
        module: ModuleId,
        layout: mir::TargetLayout,
        isa: &dyn TargetIsa,
    ) -> Result<(), EmitError> {
        let host = Triple::host();
        let target = isa.triple();
        let pointer_bytes = isa.pointer_type().bytes() as u8;
        let endian = target
            .endianness()
            .map_err(|_| Self::internal(module, "native target byte order is unknown"))?;
        let is_little = endian == Endianness::Little;
        if target != &host
            || layout.pointer_bytes() != pointer_bytes
            || layout.endian.is_little() != is_little
            || !is_little
        {
            return Err(Self::internal(
                module,
                format!("native runtime ABI cannot emit target {target} from host {host}"),
            ));
        }

        Ok(())
    }

    /// Return the exact enabled ISA features required by emitted code.
    fn features(isa: &dyn TargetIsa) -> Vec<String> {
        isa.isa_flags()
            .iter()
            .filter(|feature| feature.as_bool() == Some(true))
            .map(|feature| feature.name.to_string())
            .collect()
    }

    /// Convert MIR linkage into object linkage.
    const fn linkage(linkage: mir::Linkage) -> Linkage {
        match linkage {
            mir::Linkage::Local => Linkage::Local,
            mir::Linkage::Export => Linkage::Export,
            mir::Linkage::Import => Linkage::Import,
            mir::Linkage::Shared => Linkage::Preemptible,
        }
    }

    /// Convert one Cranelift relocation kind.
    fn relocation_kind(module: ModuleId, kind: Reloc) -> Result<native::RelocationKind, EmitError> {
        match kind {
            Reloc::Abs8 => Ok(native::RelocationKind::Absolute64),
            Reloc::Abs4 => Err(Self::internal(
                module,
                format!("native image emission produced absolute relocation {kind}"),
            )),
            Reloc::X86PCRel4 | Reloc::X86CallPCRel4 | Reloc::X86CallPLTRel4 => {
                Ok(native::RelocationKind::Relative32)
            }
            Reloc::Arm64Call => Ok(native::RelocationKind::Aarch64Call26),
            Reloc::Aarch64AdrPrelPgHi21 => Ok(native::RelocationKind::Aarch64Page21),
            Reloc::Aarch64AddAbsLo12Nc => Ok(native::RelocationKind::Aarch64Low12),
            Reloc::RiscvCallPlt => Ok(native::RelocationKind::RiscvCall),
            _ => Err(Self::internal(
                module,
                format!("unsupported native relocation {kind}"),
            )),
        }
    }

    /// Convert one Cranelift library call into a native platform import.
    fn libcall(module: ModuleId, libcall: cir::LibCall) -> Result<native::Import, EmitError> {
        let import = match libcall {
            cir::LibCall::CeilF32 => native::Import::CeilF32,
            cir::LibCall::CeilF64 => native::Import::CeilF64,
            cir::LibCall::FloorF32 => native::Import::FloorF32,
            cir::LibCall::FloorF64 => native::Import::FloorF64,
            cir::LibCall::TruncF32 => native::Import::TruncF32,
            cir::LibCall::TruncF64 => native::Import::TruncF64,
            cir::LibCall::NearestF32 => native::Import::NearestF32,
            cir::LibCall::NearestF64 => native::Import::NearestF64,
            cir::LibCall::FmaF32 => native::Import::FmaF32,
            cir::LibCall::FmaF64 => native::Import::FmaF64,
            cir::LibCall::Memcpy => native::Import::Memcpy,
            cir::LibCall::Memmove => native::Import::Memmove,
            cir::LibCall::Memset => native::Import::Memset,
            cir::LibCall::Memcmp => native::Import::Memcmp,
            cir::LibCall::Probestack => {
                return Err(Self::internal(
                    module,
                    "native code emitted an out-of-line stack probe",
                ));
            }
            cir::LibCall::ElfTlsGetAddr | cir::LibCall::ElfTlsGetOffset => {
                return Err(Self::internal(
                    module,
                    "native code emitted a thread-local storage access",
                ));
            }
            cir::LibCall::X86Pshufb => {
                return Err(Self::internal(
                    module,
                    "native code emitted an architecture-specific shuffle helper",
                ));
            }
        };

        Ok(import)
    }

    /// Return the internal function symbol.
    fn body_symbol(symbol: mir::Symbol) -> String {
        format!("__tspp_body_{:016x}", symbol.raw())
    }

    /// Return the canonical entry symbol.
    fn entry_symbol(symbol: mir::Symbol) -> String {
        format!("__tspp_entry_{:016x}", symbol.raw())
    }

    /// Build one internal emission diagnostic.
    fn internal(module: ModuleId, message: impl Into<String>) -> EmitError {
        EmitError::Internal {
            anchor: module.into(),
            module,
            message: message.into(),
        }
    }
}

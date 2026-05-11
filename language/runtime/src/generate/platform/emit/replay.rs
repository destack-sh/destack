use crate::platform::model::{
    BindingEntry, BindingType, CatalogBindingProvider, CatalogBindingReplayKind,
    CatalogBindingSimulation, CatalogEffect, CatalogReplayPayload, CatalogReplayPolicy,
};
use std::collections::BTreeSet;

use super::binding::BindingWriter;
use super::codegen::ModuleCodegen;
use super::{binding_type_requires_abi, *};

impl<'a> ModuleCodegen<'a> {
    /// Report whether replay encoding one binding value needs runtime context.
    fn replay_encode_requires_context(binding_type: &BindingType) -> bool {
        match binding_type {
            BindingType::String => true,
            BindingType::StringSlice | BindingType::Slice(_) | BindingType::Array(_) => true,
            BindingType::Optional(inner) => Self::replay_encode_requires_context(inner),
            BindingType::Newtype { inner, .. } => Self::replay_encode_requires_context(inner),
            BindingType::Struct { fields, .. } => fields
                .iter()
                .any(|field| Self::replay_encode_requires_context(&field.binding_type)),
            BindingType::TaggedUnion { variants, .. } => variants
                .iter()
                .any(|variant| Self::replay_encode_requires_context(&variant.binding_type)),
            _ => false,
        }
    }

    /// Report whether replay decoding into one VM binding value needs runtime context.
    fn replay_to_vm_requires_context(binding_type: &BindingType) -> bool {
        match binding_type {
            BindingType::String => true,
            BindingType::StringSlice | BindingType::Slice(_) | BindingType::Array(_) => true,
            BindingType::Optional(inner) => Self::replay_to_vm_requires_context(inner),
            BindingType::Newtype { inner, .. } => Self::replay_to_vm_requires_context(inner),
            BindingType::Struct { fields, .. } => fields
                .iter()
                .any(|field| Self::replay_to_vm_requires_context(&field.binding_type)),
            BindingType::TaggedUnion { variants, .. } => variants
                .iter()
                .any(|variant| Self::replay_to_vm_requires_context(&variant.binding_type)),
            _ => false,
        }
    }

    /// Report whether one replay collection should iterate borrowed items directly.
    fn replay_collection_items_can_be_borrowed(binding_type: &BindingType) -> bool {
        match binding_type {
            BindingType::String | BindingType::StringSlice => true,
            BindingType::Slice(inner) | BindingType::Array(inner) => inner.is_byte_element(),
            BindingType::Newtype { inner, .. } => {
                binding_type_requires_abi(inner)
                    && Self::replay_collection_items_can_be_borrowed(inner)
            }
            _ => false,
        }
    }

    /// Render the replay result type for one binding entry.
    fn replay_result_type(&self, entry: &BindingEntry) -> String {
        let inner = self.replay_type_for_binding(&entry.return_binding);

        format!("Result<{inner}, TraceError>")
    }

    /// Collect named types referenced by replay payloads.
    fn collect_replay_type_names(&self, binding_type: &BindingType, names: &mut BTreeSet<String>) {
        match binding_type {
            BindingType::Slice(inner) | BindingType::Array(inner) => {
                self.collect_replay_type_names(inner, names);
            }
            BindingType::Optional(inner) => {
                self.collect_replay_type_names(inner, names);
            }
            BindingType::Newtype { inner, .. } => {
                if binding_type_requires_abi(inner) {
                    self.collect_replay_type_names(inner, names);
                } else if let BindingType::Newtype {
                    name,
                    domain: type_domain,
                    ..
                } = binding_type
                {
                    names.insert(self.named_type_path(type_domain, name));
                }
            }
            BindingType::Struct {
                name,
                domain: type_domain,
                fields,
            } => {
                if binding_type_requires_abi(binding_type) {
                    let replay_name = self.replay_named_struct_name(name);
                    names.insert(self.named_type_path(type_domain, replay_name.as_str()));
                } else {
                    names.insert(self.named_type_path(type_domain, name));
                }

                for field in fields {
                    self.collect_replay_type_names(&field.binding_type, names);
                }
            }
            BindingType::Enum {
                name,
                domain: type_domain,
                ..
            } => {
                names.insert(self.named_type_path(type_domain, name));
            }
            BindingType::TaggedUnion {
                name,
                domain: type_domain,
                variants,
            } => {
                let replay_name = self.replay_named_struct_name(name);
                names.insert(self.named_type_path(type_domain, replay_name.as_str()));

                for variant in variants {
                    self.collect_replay_type_names(&variant.binding_type, names);
                }
            }
            _ => {}
        }
    }

    /// Collect VM-visible named types referenced by replay payload conversion.
    fn collect_replay_vm_type_names(
        &self,
        binding_type: &BindingType,
        names: &mut BTreeSet<String>,
    ) {
        match binding_type {
            BindingType::Slice(inner) | BindingType::Array(inner) => {
                self.collect_replay_vm_type_names(inner, names);
            }
            BindingType::Optional(inner) => {
                self.collect_replay_vm_type_names(inner, names);
            }
            BindingType::Newtype {
                name,
                domain: type_domain,
                inner,
                ..
            } => {
                if binding_type_requires_abi(inner) {
                    self.collect_replay_vm_type_names(inner, names);
                } else if type_domain == self.module() {
                    names.insert(name.clone());
                }
            }
            BindingType::Struct {
                name,
                domain: type_domain,
                fields,
            } => {
                if type_domain == self.module() {
                    if binding_type_requires_abi(binding_type) {
                        names.insert(self.value_type_name(name));
                    } else {
                        names.insert(format!("{name}Vm"));
                    }
                }

                for field in fields {
                    self.collect_replay_vm_type_names(&field.binding_type, names);
                }
            }
            BindingType::Enum {
                name,
                domain: type_domain,
                ..
            } => {
                if type_domain == self.module() {
                    names.insert(name.clone());
                }
            }
            BindingType::TaggedUnion {
                name,
                domain: type_domain,
                variants,
            } => {
                if type_domain == self.module() {
                    if binding_type_requires_abi(binding_type) {
                        names.insert(self.value_type_name(name));
                    } else {
                        names.insert(name.clone());
                    }
                }

                for variant in variants {
                    self.collect_replay_vm_type_names(&variant.binding_type, names);
                }
            }
            _ => {}
        }
    }

    /// Build the generated replay struct name for one named ABI struct.
    fn replay_named_struct_name(&self, name: &str) -> String {
        let name = self.to_pascal_case(name);

        format!("{name}ReplayRecord")
    }

    /// Render replay comparison lines for one binding value.
    pub(super) fn render_replay_compare_lines(
        &self,
        binding_type: &BindingType,
        left_expr: &str,
        right_expr: &str,
        mismatch_stmt: &str,
        counter: &mut usize,
    ) -> Vec<String> {
        match binding_type {
            BindingType::Void => Vec::new(),
            BindingType::Bool
            | BindingType::Int(_)
            | BindingType::UInt(_)
            | BindingType::Float(_)
            | BindingType::Enum { .. }
            | BindingType::String => vec![format!(
                "if {left_expr} != {right_expr} {{ {mismatch_stmt} }}"
            )],
            BindingType::Optional(inner) => {
                let mut lines = Vec::new();
                let left_value = self.next_compare_ident("left_value", counter);
                let right_value = self.next_compare_ident("right_value", counter);

                lines.push(format!(
                    "if {left_expr}.is_some() != {right_expr}.is_some() {{ {mismatch_stmt} }}"
                ));
                lines.push(format!(
                    "if let (Some({left_value}), Some({right_value})) = ({left_expr}.as_ref(), {right_expr}.as_ref()) {{"
                ));
                lines.extend(
                    self.render_replay_compare_lines(
                        inner,
                        left_value.as_str(),
                        right_value.as_str(),
                        mismatch_stmt,
                        counter,
                    )
                    .into_iter()
                    .map(|line| format!("    {line}")),
                );
                lines.push("}".to_string());

                lines
            }
            BindingType::StringSlice => self.render_replay_compare_collection_lines(
                &BindingType::String,
                left_expr,
                right_expr,
                mismatch_stmt,
                counter,
            ),
            BindingType::Slice(inner) | BindingType::Array(inner) => self
                .render_replay_compare_collection_lines(
                    inner,
                    left_expr,
                    right_expr,
                    mismatch_stmt,
                    counter,
                ),
            BindingType::Newtype { inner, .. } => self.render_replay_compare_lines(
                inner,
                left_expr,
                right_expr,
                mismatch_stmt,
                counter,
            ),
            BindingType::Struct { fields, .. } => {
                let mut lines = Vec::new();

                // compare each field
                for field in fields {
                    let field_name = self.to_snake_case(&field.name);
                    let left_field = format!("{left_expr}.{field_name}");
                    let right_field = format!("{right_expr}.{field_name}");
                    lines.extend(self.render_replay_compare_lines(
                        &field.binding_type,
                        left_field.as_str(),
                        right_field.as_str(),
                        mismatch_stmt,
                        counter,
                    ));
                }

                lines
            }
            BindingType::TaggedUnion { .. } => vec![
                format!("if {left_expr} != {right_expr} {{"),
                format!("    {mismatch_stmt}"),
                "}".to_string(),
            ],
        }
    }

    /// Render replay comparison lines for one collection binding value.
    fn render_replay_compare_collection_lines(
        &self,
        inner: &BindingType,
        left_expr: &str,
        right_expr: &str,
        mismatch_stmt: &str,
        counter: &mut usize,
    ) -> Vec<String> {
        let index_var = self.next_compare_ident("index", counter);
        let left_item = self.next_compare_ident("left_item", counter);
        let right_item = self.next_compare_ident("right_item", counter);
        let mut lines = Vec::new();

        lines.push(format!("if {left_expr}.len() != {right_expr}.len() {{"));
        lines.push(format!("    {mismatch_stmt}"));
        lines.push("}".to_string());
        lines.push(format!(
            "for ({index_var}, {left_item}) in {left_expr}.iter().enumerate() {{"
        ));
        lines.push(format!(
            "    let {right_item} = &{right_expr}[{index_var}];"
        ));
        lines.extend(
            self.render_replay_compare_lines(
                inner,
                &left_item,
                &right_item,
                mismatch_stmt,
                counter,
            )
            .into_iter()
            .map(|line| format!("    {line}")),
        );
        lines.push("}".to_string());

        lines
    }

    /// Generate one unique comparison identifier.
    fn next_compare_ident(&self, prefix: &str, counter: &mut usize) -> String {
        let name = format!("{prefix}_{counter}");
        *counter += 1;

        name
    }

    /// Render replay encoding lines for one binding value.
    fn render_replay_encode_lines(
        &self,
        binding_type: &BindingType,
        name: &str,
        value_expr: &str,
    ) -> Vec<String> {
        match binding_type {
            BindingType::Void => vec![format!("let {name} = ();")],
            BindingType::Bool
            | BindingType::Int(_)
            | BindingType::UInt(_)
            | BindingType::Float(_)
            | BindingType::Enum { .. } => vec![format!("let {name} = {value_expr};")],
            BindingType::Optional(inner) => {
                let mut lines = Vec::new();
                let inner_name = format!("{name}_inner");

                lines.push(format!("let {name} = if let Some(value) = {value_expr} {{"));
                lines.extend(
                    self.render_replay_encode_lines(inner, &inner_name, "value")
                        .into_iter()
                        .map(|line| format!("    {line}")),
                );
                lines.push(format!("    Some({inner_name})"));
                lines.push("} else {".to_string());
                lines.push("    None".to_string());
                lines.push("};".to_string());

                lines
            }
            BindingType::Newtype { inner, .. } => {
                if binding_type_requires_abi(inner) {
                    let inner_name = format!("{name}_inner");
                    let inner_expr = format!("{value_expr}.0");
                    let mut lines =
                        self.render_replay_encode_lines(inner, &inner_name, inner_expr.as_str());

                    lines.push(format!("let {name} = {inner_name};"));

                    lines
                } else {
                    vec![format!("let {name} = {value_expr};")]
                }
            }
            BindingType::String => vec![
                format!("let {name} = {{"),
                format!(
                    "    let {name}_ref = context.string_ref({value_expr}).map_err(|error| RuntimeError::from(error).boxed())?;"
                ),
                format!("    {name}_ref.as_str().to_string()"),
                "};".to_string(),
            ],
            BindingType::StringSlice => {
                self.render_replay_encode_collection_lines(&BindingType::String, name, value_expr)
            }
            BindingType::Slice(inner) => {
                if inner.is_byte_element() {
                    vec![format!("let {name} = {value_expr}.read_bytes(context)?;")]
                } else {
                    self.render_replay_encode_collection_lines(inner, name, value_expr)
                }
            }
            BindingType::Array(inner) => {
                if inner.is_byte_element() {
                    vec![format!("let {name} = {value_expr}.read_bytes(context)?;")]
                } else {
                    self.render_replay_encode_collection_lines(inner, name, value_expr)
                }
            }
            BindingType::Struct {
                name: struct_name,
                domain: struct_domain,
                fields,
            } => {
                let mut lines = Vec::new();
                let struct_type = if binding_type_requires_abi(binding_type) {
                    self.replay_struct_name(struct_name)
                } else {
                    struct_name.clone()
                };
                let mut field_names = Vec::new();

                for field in fields {
                    let field_name = self.to_snake_case(&field.name);
                    let field_value = format!("{value_expr}.{field_name}");
                    let field_var = format!("{name}_{field_name}");

                    lines.extend(self.render_replay_encode_lines(
                        &field.binding_type,
                        &field_var,
                        &field_value,
                    ));
                    field_names.push((field_name, field_var));
                }

                let struct_path = self.named_type_path(struct_domain, struct_type.as_str());
                lines.push(format!("let {name} = {struct_path} {{"));

                for (field_name, field_var) in field_names {
                    lines.push(format!("    {field_name}: {field_var},"));
                }

                lines.push("};".to_string());

                lines
            }
            BindingType::TaggedUnion {
                name: union_name,
                domain: union_domain,
                variants,
            } => {
                let mut lines = Vec::new();
                let source_union = if binding_type_requires_abi(binding_type) {
                    self.tagged_union_vm_path(union_domain, union_name)
                } else {
                    self.named_type_path(union_domain, union_name)
                };
                let target_union = if binding_type_requires_abi(binding_type) {
                    let replay_name = self.replay_tagged_union_name(union_name);
                    self.named_type_path(union_domain, replay_name.as_str())
                } else {
                    self.named_type_path(union_domain, union_name)
                };

                lines.push(format!("let {name} = match {value_expr} {{"));

                for variant in variants {
                    let inner_name =
                        format!("{name}_{}", self.to_snake_case(variant.name.as_str()));
                    lines.push(format!("    {source_union}::{}(value) => {{", variant.name));
                    lines.extend(
                        self.render_replay_encode_lines(
                            &variant.binding_type,
                            &inner_name,
                            "value",
                        )
                        .into_iter()
                        .map(|line| format!("        {line}")),
                    );
                    lines.push(format!(
                        "        {target_union}::{}({inner_name})",
                        variant.name
                    ));
                    lines.push("    }".to_string());
                }

                lines.push("};".to_string());

                lines
            }
        }
    }

    /// Render replay encoding for one slice or array value.
    fn render_replay_encode_collection_lines(
        &self,
        inner: &BindingType,
        name: &str,
        value_expr: &str,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        let raw_var = format!("{name}_raw");
        let item_var = format!("{name}_item");
        let item_recorded_var = format!("{name}_item_recorded");

        lines.push(format!("let {raw_var} = {value_expr}.values(context)?;"));
        lines.push(format!(
            "let mut {name} = Vec::with_capacity({raw_var}.len());"
        ));
        lines.push(format!("for {item_var}_value in {raw_var} {{"));
        lines.extend(
            self.render_decode_value_lines(&item_var, inner, &format!("{item_var}_value"), "item")
                .into_iter()
                .map(|line| format!("    {line}")),
        );
        lines.extend(
            self.render_replay_encode_lines(inner, &item_recorded_var, &item_var)
                .into_iter()
                .map(|line| format!("    {line}")),
        );
        lines.push(format!("    {name}.push({item_recorded_var});"));
        lines.push("}".to_string());

        lines
    }

    /// Render replay decoding lines into one VM binding type.
    fn render_replay_to_vm_binding_lines(
        &self,
        binding_type: &BindingType,
        name: &str,
        value_expr: &str,
    ) -> Vec<String> {
        match binding_type {
            BindingType::Void => vec![format!("let {name} = ();")],
            BindingType::Bool
            | BindingType::Int(_)
            | BindingType::UInt(_)
            | BindingType::Float(_)
            | BindingType::Enum { .. } => vec![format!("let {name} = {value_expr};")],
            BindingType::Optional(inner) => {
                let mut lines = Vec::new();
                let inner_name = format!("{name}_inner");

                lines.push(format!("let {name} = if let Some(value) = {value_expr} {{"));
                lines.extend(
                    self.render_replay_to_vm_binding_lines(inner, &inner_name, "value")
                        .into_iter()
                        .map(|line| format!("    {line}")),
                );
                lines.push(format!("    Some({inner_name})"));
                lines.push("} else {".to_string());
                lines.push("    None".to_string());
                lines.push("};".to_string());

                lines
            }
            BindingType::Newtype {
                name: type_name,
                domain: type_domain,
                inner,
            } => {
                if binding_type_requires_abi(inner) {
                    let inner_name = format!("{name}_inner");
                    let mut lines =
                        self.render_replay_to_vm_binding_lines(inner, &inner_name, value_expr);
                    let type_path = self.newtype_abi_constructor_path(
                        type_domain,
                        type_name,
                        "platform_abi::VmAbi",
                    );

                    lines.push(format!("let {name} = {type_path}({inner_name});"));

                    lines
                } else {
                    vec![format!("let {name} = {value_expr};")]
                }
            }
            BindingType::String => vec![format!(
                "let {name} = context.string_handle({value_expr}.as_str()).map_err(Box::<RuntimeError>::from)?;"
            )],
            BindingType::StringSlice => self.render_replay_to_vm_binding_collection_lines(
                &BindingType::String,
                name,
                value_expr,
                false,
                false,
            ),
            BindingType::Slice(inner) => {
                let is_bytes = inner.is_byte_element();
                self.render_replay_to_vm_binding_collection_lines(
                    inner, name, value_expr, is_bytes, false,
                )
            }
            BindingType::Array(inner) => {
                let is_bytes = inner.is_byte_element();
                self.render_replay_to_vm_binding_collection_lines(
                    inner, name, value_expr, is_bytes, true,
                )
            }
            BindingType::Struct {
                name: struct_name,
                domain: struct_domain,
                fields,
            } => {
                let mut lines = Vec::new();
                let mut field_values = Vec::new();

                for field in fields {
                    let field_name = self.to_snake_case(&field.name);
                    let field_var = format!("{name}_{field_name}");
                    let field_expr = format!("{value_expr}.{field_name}");

                    lines.extend(self.render_replay_to_vm_binding_lines(
                        &field.binding_type,
                        &field_var,
                        &field_expr,
                    ));
                    field_values.push((field_name, field_var));
                }

                // vm-facing ABI structs rebuild through their VM alias
                let struct_path = if binding_type_requires_abi(binding_type) {
                    self.struct_vm_path(struct_domain, struct_name)
                } else {
                    self.named_type_path(struct_domain, struct_name)
                };
                lines.push(format!("let {name} = {struct_path} {{"));

                for (field_name, field_var) in field_values {
                    lines.push(format!("    {field_name}: {field_var},"));
                }

                lines.push("};".to_string());

                lines
            }
            BindingType::TaggedUnion {
                name: union_name,
                domain: union_domain,
                variants,
            } => {
                let mut lines = Vec::new();
                let source_union = if binding_type_requires_abi(binding_type) {
                    let replay_name = self.replay_tagged_union_name(union_name);
                    self.named_type_path(union_domain, replay_name.as_str())
                } else {
                    self.named_type_path(union_domain, union_name)
                };
                let target_union = if binding_type_requires_abi(binding_type) {
                    self.tagged_union_vm_path(union_domain, union_name)
                } else {
                    self.named_type_path(union_domain, union_name)
                };

                lines.push(format!("let {name} = match {value_expr} {{"));

                for variant in variants {
                    let inner_name =
                        format!("{name}_{}", self.to_snake_case(variant.name.as_str()));
                    lines.push(format!("    {source_union}::{}(value) => {{", variant.name));
                    lines.extend(
                        self.render_replay_to_vm_binding_lines(
                            &variant.binding_type,
                            &inner_name,
                            "value",
                        )
                        .into_iter()
                        .map(|line| format!("        {line}")),
                    );
                    lines.push(format!(
                        "        {target_union}::{}({inner_name})",
                        variant.name
                    ));
                    lines.push("    }".to_string());
                }

                lines.push("};".to_string());

                lines
            }
        }
    }

    /// Render replay decoding for one slice or array into one VM binding type.
    fn render_replay_to_vm_binding_collection_lines(
        &self,
        inner: &BindingType,
        name: &str,
        value_expr: &str,
        is_bytes: bool,
        is_array: bool,
    ) -> Vec<String> {
        let mut lines = Vec::new();

        if is_bytes {
            if is_array {
                lines.push(format!(
                    "let {name} = VmArray::<u8>::from_bytes(context, {value_expr}.as_ref())?;"
                ));
            } else {
                lines.push(format!(
                    "let {name} = VmSlice::<u8>::from_bytes(context, {value_expr}.as_ref())?;"
                ));
            }

            return lines;
        }

        let builder_var = format!("{name}_builder");
        let item_var = format!("{name}_item");
        let item_value_var = format!("{name}_item_value");
        let inner_type = self.vm_type_for_binding(inner);

        // replay payloads are owned, so rebuild VM collections directly in final VM storage
        if is_array {
            lines.push(format!(
                "let mut {builder_var} = VmArray::<{inner_type}>::builder(context, {value_expr}.len())?;"
            ));
        } else {
            lines.push(format!(
                "let mut {builder_var} = VmSlice::<{inner_type}>::builder(context, {value_expr}.len())?;"
            ));
        }

        lines.push(format!("for {item_var} in {value_expr} {{"));
        lines.extend(
            self.render_replay_to_vm_binding_lines(inner, &item_value_var, &item_var)
                .into_iter()
                .map(|line| format!("    {line}")),
        );
        lines.push(format!(
            "    {builder_var}.push(context, {item_value_var})?;"
        ));
        lines.push("}".to_string());
        lines.push(format!("let {name} = {builder_var}.finish()?;"));

        lines
    }

    /// Render native replay encoding lines for one binding value.
    fn render_native_replay_encode_lines(
        &self,
        binding_type: &BindingType,
        name: &str,
        value_expr: &str,
    ) -> Vec<String> {
        match binding_type {
            BindingType::Void => vec![format!("let {name} = ();")],
            BindingType::Bool
            | BindingType::Int(_)
            | BindingType::UInt(_)
            | BindingType::Float(_)
            | BindingType::Enum { .. } => vec![format!("let {name} = {value_expr};")],
            BindingType::String => vec![format!(
                "let {name} = unsafe {{ {value_expr}.as_str()? }}.to_string();"
            )],
            BindingType::StringSlice => {
                self.render_native_replay_encode_string_slice_lines(name, value_expr)
            }
            BindingType::Slice(inner) | BindingType::Array(inner) => {
                self.render_native_replay_encode_collection_lines(inner, name, value_expr)
            }
            BindingType::Optional(inner) => {
                let mut lines = Vec::new();
                let inner_name = format!("{name}_inner");

                lines.push(format!("let {name} = if let Some(value) = {value_expr} {{"));
                lines.extend(
                    self.render_native_replay_encode_lines(inner, &inner_name, "value")
                        .into_iter()
                        .map(|line| format!("    {line}")),
                );
                lines.push(format!("    Some({inner_name})"));
                lines.push("} else {".to_string());
                lines.push("    None".to_string());
                lines.push("};".to_string());

                lines
            }
            BindingType::Newtype { inner, .. } => {
                if binding_type_requires_abi(inner) {
                    let inner_name = format!("{name}_inner");
                    let inner_expr = format!("{value_expr}.0");
                    let mut lines = self.render_native_replay_encode_lines(
                        inner,
                        &inner_name,
                        inner_expr.as_str(),
                    );

                    lines.push(format!("let {name} = {inner_name};"));

                    lines
                } else {
                    vec![format!("let {name} = {value_expr};")]
                }
            }
            BindingType::Struct {
                name: struct_name,
                domain: struct_domain,
                fields,
            } => {
                let mut lines = Vec::new();
                let struct_type = if binding_type_requires_abi(binding_type) {
                    let replay_name = self.replay_named_struct_name(struct_name);
                    self.named_type_path(struct_domain, replay_name.as_str())
                } else {
                    self.named_type_path(struct_domain, struct_name)
                };
                let mut field_names = Vec::new();

                // encode each field
                for field in fields {
                    let field_name = self.to_snake_case(&field.name);
                    let field_value = format!("{value_expr}.{field_name}");
                    let field_var = format!("{name}_{field_name}");

                    lines.extend(self.render_native_replay_encode_lines(
                        &field.binding_type,
                        &field_var,
                        &field_value,
                    ));
                    field_names.push((field_name, field_var));
                }

                lines.push(format!("let {name} = {struct_type} {{"));

                for (field_name, field_var) in field_names {
                    lines.push(format!("    {field_name}: {field_var},"));
                }

                lines.push("};".to_string());

                lines
            }
            BindingType::TaggedUnion {
                name: union_name,
                domain: union_domain,
                variants,
            } => {
                let mut lines = Vec::new();
                let source_union = self.named_type_path(union_domain, union_name);
                let target_union = if binding_type_requires_abi(binding_type) {
                    let replay_name = self.replay_named_struct_name(union_name);
                    self.named_type_path(union_domain, replay_name.as_str())
                } else {
                    self.named_type_path(union_domain, union_name)
                };

                lines.push(format!("let {name} = match {value_expr} {{"));

                // encode each union variant
                for variant in variants {
                    let inner_name = format!("{name}_{}", self.to_snake_case(&variant.name));
                    lines.push(format!("    {source_union}::{}(value) => {{", variant.name));
                    lines.extend(
                        self.render_native_replay_encode_lines(
                            &variant.binding_type,
                            &inner_name,
                            "value",
                        )
                        .into_iter()
                        .map(|line| format!("        {line}")),
                    );
                    lines.push(format!(
                        "        {target_union}::{}({inner_name})",
                        variant.name
                    ));
                    lines.push("    }".to_string());
                }

                lines.push("};".to_string());

                lines
            }
        }
    }

    /// Render native replay encoding lines for one string slice.
    fn render_native_replay_encode_string_slice_lines(
        &self,
        name: &str,
        value_expr: &str,
    ) -> Vec<String> {
        let item_var = format!("{name}_item");
        let item_recorded_var = format!("{name}_item_recorded");
        let mut lines = Vec::new();

        lines.push(format!(
            "let {name}_slice = unsafe {{ {value_expr}.as_slice()? }};"
        ));
        lines.push(format!(
            "let mut {name} = Vec::with_capacity({name}_slice.len());"
        ));
        lines.push(format!("for {item_var} in {name}_slice.iter() {{"));
        lines.push(format!(
            "    let {item_recorded_var} = unsafe {{ {item_var}.as_str()? }}.to_string();"
        ));
        lines.push(format!("    {name}.push({item_recorded_var});"));
        lines.push("}".to_string());

        lines
    }

    /// Render native replay encoding lines for one collection binding value.
    fn render_native_replay_encode_collection_lines(
        &self,
        inner: &BindingType,
        name: &str,
        value_expr: &str,
    ) -> Vec<String> {
        let item_var = format!("{name}_item");
        let item_recorded_var = format!("{name}_item_recorded");
        let borrow_items = Self::replay_collection_items_can_be_borrowed(inner);
        let mut lines = Vec::new();

        lines.push(format!(
            "let {name}_slice = unsafe {{ {value_expr}.as_slice()? }};"
        ));
        lines.push(format!(
            "let mut {name} = Vec::with_capacity({name}_slice.len());"
        ));
        if borrow_items {
            lines.push(format!("for {item_var} in {name}_slice.iter() {{"));
        } else {
            lines.push(format!("for {item_var} in {name}_slice.iter().cloned() {{"));
        }
        lines.extend(
            self.render_native_replay_encode_lines(inner, &item_recorded_var, item_var.as_str())
                .into_iter()
                .map(|line| format!("    {line}")),
        );
        lines.push(format!("    {name}.push({item_recorded_var});"));
        lines.push("}".to_string());

        lines
    }

    /// Render native replay read out lines for one binding value.
    fn render_native_replay_read_out_lines(
        &self,
        binding_type: &BindingType,
        out_expr: &str,
        name: &str,
    ) -> Vec<String> {
        match binding_type {
            BindingType::Void => vec![],
            _ => {
                let native_type = self.native_type_for_binding(binding_type);
                vec![format!(
                    "let {name}: {native_type} = unsafe {{ {out_expr}.read() }};"
                )]
            }
        }
    }

    /// Render native replay out-store lines for one binding value.
    fn render_native_replay_store_lines(
        &self,
        binding_type: &BindingType,
        out_expr: &str,
        value_expr: &str,
        name: &str,
    ) -> Vec<String> {
        if Self::binding_type_supports_direct_replay_store(binding_type) {
            return vec![format!("unsafe {{ {out_expr}.write({value_expr}) }};")];
        }

        if matches!(binding_type, BindingType::Void) {
            return Vec::new();
        }

        let mut lines = self.render_native_replay_decode_lines(binding_type, name, value_expr);
        lines.push(format!("unsafe {{ {out_expr}.write({name}) }};"));

        lines
    }

    /// Render native replay decode lines for one replay payload value.
    fn render_native_replay_decode_lines(
        &self,
        binding_type: &BindingType,
        name: &str,
        value_expr: &str,
    ) -> Vec<String> {
        match binding_type {
            BindingType::Void => vec![format!("let {name} = ();")],
            BindingType::Bool
            | BindingType::Int(_)
            | BindingType::UInt(_)
            | BindingType::Float(_)
            | BindingType::Enum { .. } => vec![format!("let {name} = {value_expr};")],
            BindingType::String => {
                vec![format!(
                    "let {name} = binding.store_string_owned({value_expr});"
                )]
            }
            BindingType::StringSlice => {
                let mut lines = Vec::new();
                lines.push(format!(
                    "let {name} = binding.store_string_slice_with({value_expr}.len(), |{name}_values| {{"
                ));
                lines.push(format!("    for value in {value_expr} {{"));
                lines.push("        let value = binding.store_string_owned(value);".to_string());
                lines.push(format!("        {name}_values.push(value);"));
                lines.push("    }".to_string());
                lines.push("    Ok(())".to_string());
                lines.push("})?;".to_string());
                lines
            }
            BindingType::Slice(inner) => {
                self.render_native_replay_decode_collection_lines(inner, name, value_expr, false)
            }
            BindingType::Array(inner) => {
                self.render_native_replay_decode_collection_lines(inner, name, value_expr, true)
            }
            BindingType::Optional(inner) => {
                let mut lines = Vec::new();
                let inner_name = format!("{name}_inner");

                lines.push(format!("let {name} = if let Some(value) = {value_expr} {{"));
                lines.extend(
                    self.render_native_replay_decode_lines(inner, &inner_name, "value")
                        .into_iter()
                        .map(|line| format!("    {line}")),
                );
                lines.push(format!("    Some({inner_name})"));
                lines.push("} else {".to_string());
                lines.push("    None".to_string());
                lines.push("};".to_string());

                lines
            }
            BindingType::Newtype {
                name: type_name,
                domain: type_domain,
                inner,
            } => {
                if binding_type_requires_abi(inner) {
                    let inner_name = format!("{name}_inner");
                    let mut lines =
                        self.render_native_replay_decode_lines(inner, &inner_name, value_expr);
                    let type_path = self.newtype_abi_constructor_path(
                        type_domain,
                        type_name,
                        "platform_abi::NativeAbi",
                    );
                    lines.push(format!("let {name} = {type_path}({inner_name});"));
                    lines
                } else {
                    vec![format!("let {name} = {value_expr};")]
                }
            }
            BindingType::Struct {
                name: struct_name,
                domain: struct_domain,
                fields,
            } => {
                let mut lines = Vec::new();
                let struct_type = self.named_type_path(struct_domain, struct_name);
                let mut field_names = Vec::new();

                // decode each field
                for field in fields {
                    let field_name = self.to_snake_case(&field.name);
                    let field_value = format!("{value_expr}.{field_name}");
                    let field_var = format!("{name}_{field_name}");

                    lines.extend(self.render_native_replay_decode_lines(
                        &field.binding_type,
                        &field_var,
                        &field_value,
                    ));
                    field_names.push((field_name, field_var));
                }

                lines.push(format!("let {name} = {struct_type} {{"));

                for (field_name, field_var) in field_names {
                    lines.push(format!("    {field_name}: {field_var},"));
                }

                lines.push("};".to_string());

                lines
            }
            BindingType::TaggedUnion {
                name: union_name,
                domain: union_domain,
                variants,
            } => {
                let mut lines = Vec::new();
                let source_union = if binding_type_requires_abi(binding_type) {
                    let replay_name = self.replay_named_struct_name(union_name);
                    self.named_type_path(union_domain, replay_name.as_str())
                } else {
                    self.named_type_path(union_domain, union_name)
                };
                let target_union = self.named_type_path(union_domain, union_name);

                lines.push(format!("let {name} = match {value_expr} {{"));

                // decode each union variant
                for variant in variants {
                    let inner_name = format!("{name}_{}", self.to_snake_case(&variant.name));
                    lines.push(format!("    {source_union}::{}(value) => {{", variant.name));
                    lines.extend(
                        self.render_native_replay_decode_lines(
                            &variant.binding_type,
                            &inner_name,
                            "value",
                        )
                        .into_iter()
                        .map(|line| format!("        {line}")),
                    );
                    lines.push(format!(
                        "        {target_union}::{}({inner_name})",
                        variant.name
                    ));
                    lines.push("    }".to_string());
                }

                lines.push("};".to_string());

                lines
            }
        }
    }

    /// Render native replay decode lines for one collection payload.
    fn render_native_replay_decode_collection_lines(
        &self,
        inner: &BindingType,
        name: &str,
        value_expr: &str,
        is_array: bool,
    ) -> Vec<String> {
        let item_var = format!("{name}_item");
        let item_decoded_var = format!("{name}_decoded");
        let mut lines = Vec::new();

        if is_array {
            lines.push(format!(
                "let {name} = binding.store_array_with({value_expr}.len(), |{name}_values| {{"
            ));
        } else {
            lines.push(format!(
                "let {name} = binding.store_slice_with({value_expr}.len(), |{name}_values| {{"
            ));
        }
        lines.push(format!("    for {item_var} in {value_expr} {{"));
        lines.extend(
            self.render_native_replay_decode_lines(inner, &item_decoded_var, item_var.as_str())
                .into_iter()
                .map(|line| format!("        {line}")),
        );
        lines.push(format!("        {name}_values.push({item_decoded_var});"));
        lines.push("    }".to_string());
        lines.push("    Ok(())".to_string());
        lines.push("})?;".to_string());

        lines
    }

    /// Report whether one binding type can be written directly during native replay restore.
    fn binding_type_supports_direct_replay_store(binding_type: &BindingType) -> bool {
        match binding_type {
            BindingType::Void
            | BindingType::Bool
            | BindingType::Int(_)
            | BindingType::UInt(_)
            | BindingType::Float(_)
            | BindingType::Enum { .. } => true,
            BindingType::Optional(inner) => Self::binding_type_supports_direct_replay_store(inner),
            BindingType::Newtype { inner, .. } => !binding_type_requires_abi(inner),
            BindingType::String
            | BindingType::StringSlice
            | BindingType::Slice(_)
            | BindingType::Array(_)
            | BindingType::Struct { .. }
            | BindingType::TaggedUnion { .. } => false,
        }
    }
}

impl<'spec, 'output> BindingWriter<'spec, 'output> {
    /// Render replay payload structs for bindings.
    pub(super) fn write_replay_payloads(&mut self) {
        let output = &mut self.output;
        let codegen = self.spec.codegen();
        let consts = &self.spec.consts;

        let mut wrote = false;
        for binding in consts {
            let entry = binding.entry;
            let CatalogEffect::External {
                replay: CatalogReplayPolicy::Recordable,
            } = entry.effect
            else {
                continue;
            };
            if entry.replay_kind != CatalogBindingReplayKind::BindingCall {
                continue;
            }

            let struct_name = codegen.replay_struct_name(&binding.const_name);
            let args_struct_name = format!("{struct_name}Args");
            let result_type = codegen.replay_result_type(entry);
            let supports_args = matches!(
                entry.replay_payload,
                CatalogReplayPayload::ArgumentsAndResults
            );
            if supports_args {
                output.push_str(&format!(
                    "/// Replay argument payload for {}.\n",
                    binding.extern_name
                ));
                output.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
                output.push_str(&format!("struct {args_struct_name} {{\n"));
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = codegen.sanitize_param_name(&param.name, index);
                    let field_type = codegen.replay_type_for_binding(&param.binding_type);
                    output.push_str(&format!("    /// Replay value for {name}.\n"));
                    output.push_str(&format!("    pub {name}: {field_type},\n"));
                }
                output.push_str("}\n\n");
            }
            output.push_str(&format!(
                "/// Replay payload for {}.\n",
                binding.extern_name
            ));
            output.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
            output.push_str(&format!("struct {struct_name} {{\n"));
            if supports_args {
                output.push_str("    /// Optional replay arguments.\n");
                output.push_str(&format!("    pub args: Option<{args_struct_name}>,\n"));
            }
            output.push_str("    /// Replay result payload.\n");
            output.push_str(&format!("    pub result: {result_type},\n"));
            output.push_str("}\n\n");
            wrote = true;
        }

        if wrote {
            output.push_str("\n");
        }
    }

    /// Render the VM replay helpers for bindings.
    pub(super) fn write_vm_replay_helpers(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.module;
        let consts = &self.spec.consts;

        let bindings = consts.iter().filter(|binding| {
            matches!(
                binding.entry.effect,
                CatalogEffect::External {
                    replay: CatalogReplayPolicy::Recordable
                }
            ) && binding.entry.replay_kind == CatalogBindingReplayKind::BindingCall
        });

        let mut emitted_header = false;
        for binding in bindings {
            if !emitted_header {
                output.push_str(&format!(
                    "/// VM replay implementations for {domain} bindings.\n"
                ));
                emitted_header = true;
            }

            let entry = binding.entry;
            let codegen = ModuleCodegen::new(domain);
            let fn_name = codegen.vm_replay_fn_name(binding.extern_name);
            let helper_base = codegen.vm_fn_name(binding.extern_name);
            let implementation_fn_name = &binding.implementation_fn_name;
            let encode_helper = codegen.encode_helper_name(&helper_base);
            let supports_args = matches!(
                entry.replay_payload,
                CatalogReplayPayload::ArgumentsAndResults
            );
            let replay_args_use_context = supports_args
                && entry.parameters.iter().any(|param| {
                    ModuleCodegen::replay_encode_requires_context(&param.binding_type)
                });
            let replay_result_use_context = !matches!(entry.return_binding, BindingType::Void)
                && ModuleCodegen::replay_encode_requires_context(&entry.return_binding);
            let replay_record_uses_context = replay_args_use_context || replay_result_use_context;
            let replay_restore_uses_context = supports_args && replay_args_use_context
                || !matches!(entry.return_binding, BindingType::Void)
                    && ModuleCodegen::replay_to_vm_requires_context(&entry.return_binding);
            let replay_record_context_name = if replay_record_uses_context {
                "context"
            } else {
                "_context"
            };
            let replay_restore_context_name = if replay_restore_uses_context {
                "context"
            } else {
                "_context"
            };
            let replay_struct = codegen.replay_struct_name(&binding.const_name);
            let replay_args_struct = format!("{replay_struct}Args");
            let invoke_args = codegen.render_invoke_args_with_prefix(entry);

            let mut params = Vec::new();
            for (index, param) in entry.parameters.iter().enumerate() {
                let name = codegen.sanitize_param_name(&param.name, index);
                let ty = codegen.vm_type_for_binding(&param.binding_type);
                params.push(format!("{name}: {ty}"));
            }

            output.push_str("#[inline]\n");
            output.push_str(&format!("fn {fn_name}(\n"));
            output.push_str("    binding: &BindingCallContext,\n");
            output.push_str("    context: &mut vm::BindingContext<'_>,\n");
            if entry.provider != CatalogBindingProvider::Runtime {
                output.push_str("    world: RuntimeWorld,\n");
            }
            for param in &params {
                output.push_str(&format!("    {param},\n"));
            }
            output.push_str(") -> RuntimeResult<vm::Word> {\n");

            output.push_str("    let result = binding.trace().run_binding(\n");
            output.push_str(&format!("        {},\n", binding.const_name));
            output.push_str(&format!(
                "        binding.replay_payload_for({})?,\n",
                binding.const_name
            ));
            output.push_str("        context,\n");
            if entry.provider == CatalogBindingProvider::Runtime {
                output.push_str(&format!(
                    "        |context| platform_runtime_vm::{}(binding, context{invoke_args}),\n",
                    implementation_fn_name
                ));
            } else {
                let simulation_call = match entry.simulation {
                    CatalogBindingSimulation::Unsupported => format!(
                        "Err(RuntimeError::from(PlatformError::not_supported({}.name)).boxed())",
                        binding.const_name
                    ),
                    CatalogBindingSimulation::Supported => {
                        format!(
                            "platform_simulation_vm::{}(binding, context{invoke_args})",
                            implementation_fn_name
                        )
                    }
                };
                output.push_str("        |context| {\n");
                output.push_str("            match world {\n");
                output.push_str(&format!(
                    "                RuntimeWorld::Host => platform_vm::{}(binding, context{invoke_args}),\n",
                    implementation_fn_name
                ));
                output.push_str(&format!(
                    "                RuntimeWorld::Simulation => {simulation_call},\n"
                ));
                output.push_str("            }\n");
                output.push_str("        },\n");
            }
            output.push_str(&format!(
                "        |{replay_record_context_name}, result| {{\n"
            ));
            if replay_record_uses_context {
                output.push_str("            let context = &context.read();\n");
            }
            if supports_args {
                output.push_str("            let record_args = matches!(\n");
                output.push_str(&format!(
                    "                binding.replay_payload_for({})?,\n",
                    binding.const_name
                ));
                output.push_str("                BindingReplayPayload::ArgumentsAndResults,\n");
                output.push_str("            );\n");
                output.push_str("            let args = if record_args {\n");
                output.push_str("                // replay args\n");
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = codegen.sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    for line in codegen.render_replay_encode_lines(
                        &param.binding_type,
                        &recorded_name,
                        &name,
                    ) {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                output.push_str(&format!("                Some({replay_args_struct} {{\n"));
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = codegen.sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    output.push_str(&format!("                    {name}: {recorded_name},\n"));
                }
                output.push_str("                })\n");
                output.push_str("            } else {\n");
                output.push_str("                None\n");
                output.push_str("            };\n\n");
            }

            if matches!(entry.return_binding, BindingType::Void) {
                output.push_str("            if let Ok(()) = result {\n");
                output.push_str("                let result_recorded = ();\n");
            } else {
                output.push_str("            if let Ok(value) = result {\n");
                let vm_result_type = codegen.vm_type_for_binding(&entry.return_binding);
                output.push_str(&format!(
                    "                let result_value: {vm_result_type} = value.clone();\n"
                ));
                for line in codegen.render_replay_encode_lines(
                    &entry.return_binding,
                    "result_recorded",
                    "result_value",
                ) {
                    output.push_str(&format!("                {line}\n"));
                }
            }
            output.push_str(&format!(
                "                let payload = {replay_struct} {{\n"
            ));
            if supports_args {
                output.push_str("                    args,\n");
            }
            output.push_str("                    result: Ok(result_recorded),\n");
            output.push_str("                };\n");
            output.push_str("                return Ok(Some(payload));\n");
            output.push_str("            }\n\n");

            output.push_str("            if let Err(error) = result {\n");
            output.push_str("                let payload = {\n");
            output.push_str(
                "                    let result = Err(TraceError::from(error.as_ref()));\n",
            );
            output.push_str(&format!("                    {replay_struct} {{\n"));
            if supports_args {
                output.push_str("                        args,\n");
            }
            output.push_str("                        result,\n");
            output.push_str("                    }\n");
            output.push_str("                };\n");
            output.push_str("                return Ok(Some(payload));\n");
            output.push_str("            }\n\n");

            output.push_str("            Ok(None)\n");
            output.push_str("        },\n");
            output.push_str(&format!(
                "        |{replay_restore_context_name}, payload| {{\n"
            ));
            if replay_restore_uses_context {
                output.push_str("            let context = &mut context.write();\n");
            }
            if supports_args {
                output.push_str("            if let Some(args) = payload.args.as_ref() {\n");
                output.push_str("                // replay arg verification\n");
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = codegen.sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    for line in codegen.render_replay_encode_lines(
                        &param.binding_type,
                        &recorded_name,
                        &name,
                    ) {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                let mut compare_counter = 0usize;
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = codegen.sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    output.push_str(&format!(
                        "                let {name}_payload = &args.{name};\n"
                    ));
                    output.push_str(&format!(
                        "                let {name}_current = &{recorded_name};\n"
                    ));
                    let mismatch_stmt = format!(
                        "return Err(RuntimeError::TraceMismatch {{ name: {}.name.to_string() }}.boxed());",
                        binding.const_name
                    );
                    let compare_lines = codegen.render_replay_compare_lines(
                        &param.binding_type,
                        &format!("{name}_payload"),
                        &format!("{name}_current"),
                        &mismatch_stmt,
                        &mut compare_counter,
                    );
                    for line in compare_lines {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                output.push_str("            }\n\n");
            }

            output.push_str("            // replay result\n");
            output.push_str("            match payload.result {\n");
            if entry.return_binding != BindingType::Void {
                output.push_str("                Ok(value) => {\n");
                for line in codegen.render_replay_to_vm_binding_lines(
                    &entry.return_binding,
                    "vm_result",
                    "value",
                ) {
                    output.push_str(&format!("                    {line}\n"));
                }
                output.push_str("                    Ok(vm_result)\n");
                output.push_str("                }\n");
            } else {
                output.push_str("                Ok(()) => Ok(()),\n");
            }
            output
                .push_str("                Err(error) => Err(Box::<RuntimeError>::from(error)),\n");
            output.push_str("            }\n");
            output.push_str("        },\n");
            output.push_str("    );\n");
            output.push_str(&format!(
                "    let result = {encode_helper}(context, result)?;\n"
            ));
            output.push_str("    Ok(result)\n");
            output.push_str("}\n\n");
        }
    }

    /// Render native replay helpers for a domain.
    pub(crate) fn write_native_replay_helpers(&mut self) {
        let output = &mut self.output;
        let codegen = self.spec.codegen();
        let domain = codegen.module();
        let consts = &self.spec.consts;

        let bindings = consts.iter().filter(|binding| {
            matches!(
                binding.entry.effect,
                CatalogEffect::External {
                    replay: CatalogReplayPolicy::Recordable
                }
            ) && binding.entry.replay_kind == CatalogBindingReplayKind::BindingCall
        });

        let mut emitted_header = false;
        for binding in bindings {
            if !emitted_header {
                output.push_str(&format!(
                    "/// Native replay implementations for {domain} bindings.\n"
                ));
                emitted_header = true;
            }

            let entry = binding.entry;
            let fn_name = codegen.native_replay_fn_name(binding.extern_name);
            let implementation_fn_name = &binding.implementation_fn_name;
            let supports_args = matches!(
                entry.replay_payload,
                CatalogReplayPayload::ArgumentsAndResults
            );
            let replay_struct = codegen.replay_struct_name(&binding.const_name);
            let replay_args_struct = format!("{replay_struct}Args");

            let mut params = Vec::new();
            let mut args = Vec::new();
            if entry.return_binding != BindingType::Void {
                let out_type = codegen.native_type_for_binding(&entry.return_binding);
                params.push(format!("out: *mut {out_type}"));
                args.push("out".to_string());
            }
            for (index, param) in entry.parameters.iter().enumerate() {
                let name = codegen.sanitize_param_name(&param.name, index);
                let ty = codegen.native_type_for_binding(&param.binding_type);
                params.push(format!("{name}: {ty}"));
                args.push(name);
            }

            output.push_str("#[inline]\n");
            output.push_str(&format!("fn {fn_name}(\n"));
            output.push_str("    binding: &BindingCallContext,\n");
            if entry.provider != CatalogBindingProvider::Runtime {
                output.push_str("    world: RuntimeWorld,\n");
            }
            for param in &params {
                output.push_str(&format!("    {param},\n"));
            }
            output.push_str(") -> RuntimeResult<()> {\n");

            if !supports_args && !entry.parameters.is_empty() {
                let unused = entry
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, param)| codegen.sanitize_param_name(&param.name, index))
                    .collect::<Vec<_>>();
                if unused.len() == 1 {
                    output.push_str(&format!("    let _ = &{};\n\n", unused[0]));
                } else {
                    output.push_str("    let _ = (");
                    output.push_str(
                        &unused
                            .iter()
                            .map(|name| format!("&{name}"))
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    output.push_str(");\n\n");
                }
            }

            output.push_str("    binding.trace().run_binding_without_context(\n");
            output.push_str(&format!("        {},\n", binding.const_name));
            output.push_str(&format!(
                "        binding.replay_payload_for({})?,\n",
                binding.const_name
            ));
            let runtime_call = if args.is_empty() {
                format!(
                    "unsafe {{ platform_runtime_native::{}(binding) }}",
                    implementation_fn_name
                )
            } else {
                format!(
                    "unsafe {{ platform_runtime_native::{}(binding, {}) }}",
                    implementation_fn_name,
                    args.join(", ")
                )
            };
            let host_call = if args.is_empty() {
                format!(
                    "unsafe {{ platform_native::{}(binding) }}",
                    implementation_fn_name
                )
            } else {
                format!(
                    "unsafe {{ platform_native::{}(binding, {}) }}",
                    implementation_fn_name,
                    args.join(", ")
                )
            };
            if entry.provider == CatalogBindingProvider::Runtime {
                output.push_str(&format!("        || {runtime_call},\n"));
            } else {
                let simulation_call = match entry.simulation {
                    CatalogBindingSimulation::Unsupported => format!(
                        "Err(RuntimeError::from(PlatformError::not_supported({}.name)).boxed())",
                        binding.const_name
                    ),
                    CatalogBindingSimulation::Supported => {
                        if args.is_empty() {
                            format!(
                                "unsafe {{ platform_simulation_native::{}(binding) }}",
                                implementation_fn_name
                            )
                        } else {
                            format!(
                                "unsafe {{ platform_simulation_native::{}(binding, {}) }}",
                                implementation_fn_name,
                                args.join(", ")
                            )
                        }
                    }
                };
                output.push_str("        || match world {\n");
                output.push_str(&format!("            RuntimeWorld::Host => {host_call},\n"));
                output.push_str(&format!(
                    "            RuntimeWorld::Simulation => {simulation_call},\n"
                ));
                output.push_str("        },\n");
            }
            output.push_str("        |result| {\n");

            if supports_args {
                output.push_str("            let record_args = matches!(\n");
                output.push_str(&format!(
                    "                binding.replay_payload_for({})?,\n",
                    binding.const_name
                ));
                output.push_str("                BindingReplayPayload::ArgumentsAndResults,\n");
                output.push_str("            );\n");
                output.push_str("            let args = if record_args {\n");
                output.push_str("                // replay args\n");
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = codegen.sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    for line in codegen.render_native_replay_encode_lines(
                        &param.binding_type,
                        &recorded_name,
                        &name,
                    ) {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                output.push_str(&format!("                Some({replay_args_struct} {{\n"));
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = codegen.sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    output.push_str(&format!("                    {name}: {recorded_name},\n"));
                }
                output.push_str("                })\n");
                output.push_str("            } else {\n");
                output.push_str("                None\n");
                output.push_str("            };\n\n");
            }

            output.push_str("            if let Ok(()) = result {\n");
            if entry.return_binding != BindingType::Void {
                for line in codegen.render_native_replay_read_out_lines(
                    &entry.return_binding,
                    "out",
                    "result_value",
                ) {
                    output.push_str(&format!("                {line}\n"));
                }
                for line in codegen.render_native_replay_encode_lines(
                    &entry.return_binding,
                    "result_recorded",
                    "result_value",
                ) {
                    output.push_str(&format!("                {line}\n"));
                }
            } else {
                output.push_str("                let result_recorded = ();\n");
            }
            output.push_str(&format!(
                "                let payload = {replay_struct} {{\n"
            ));
            if supports_args {
                output.push_str("                    args,\n");
            }
            output.push_str("                    result: Ok(result_recorded),\n");
            output.push_str("                };\n");
            output.push_str("                return Ok(Some(payload));\n");
            output.push_str("            }\n\n");

            output.push_str("            if let Err(error) = result {\n");
            output.push_str("                let payload = {\n");
            output.push_str(
                "                    let result = Err(TraceError::from(error.as_ref()));\n",
            );
            output.push_str(&format!("                    {replay_struct} {{\n"));
            if supports_args {
                output.push_str("                        args,\n");
            }
            output.push_str("                        result,\n");
            output.push_str("                    }\n");
            output.push_str("                };\n");
            output.push_str("                return Ok(Some(payload));\n");
            output.push_str("            }\n\n");

            output.push_str("            Ok(None)\n");
            output.push_str("        },\n");
            output.push_str("        |payload| {\n");

            if supports_args {
                output.push_str("            if let Some(args) = payload.args.as_ref() {\n");
                output.push_str("                // replay arg verification\n");
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = codegen.sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    for line in codegen.render_native_replay_encode_lines(
                        &param.binding_type,
                        &recorded_name,
                        &name,
                    ) {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                let mut compare_counter = 0usize;
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = codegen.sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    output.push_str(&format!(
                        "                let {name}_payload = &args.{name};\n"
                    ));
                    output.push_str(&format!(
                        "                let {name}_current = &{recorded_name};\n"
                    ));
                    let mismatch_stmt = format!(
                        "return Err(RuntimeError::TraceMismatch {{ name: {}.name.to_string() }}.boxed());",
                        binding.const_name
                    );
                    let compare_lines = codegen.render_replay_compare_lines(
                        &param.binding_type,
                        &format!("{name}_payload"),
                        &format!("{name}_current"),
                        &mismatch_stmt,
                        &mut compare_counter,
                    );
                    for line in compare_lines {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                output.push_str("            }\n\n");
            }

            output.push_str("            // replay result\n");
            output.push_str("            match payload.result {\n");
            if entry.return_binding != BindingType::Void {
                output.push_str("                Ok(value) => {\n");
                for line in codegen.render_native_replay_store_lines(
                    &entry.return_binding,
                    "out",
                    "value",
                    "value_native",
                ) {
                    output.push_str(&format!("                    {line}\n"));
                }
                output.push_str("                    Ok(())\n");
                output.push_str("                }\n");
            } else {
                output.push_str("                Ok(()) => Ok(()),\n");
            }
            output
                .push_str("                Err(error) => Err(Box::<RuntimeError>::from(error)),\n");
            output.push_str("            }\n");
            output.push_str("        },\n");
            output.push_str("    )\n");
            output.push_str("}\n\n");
        }
    }
}

/// Collect named types referenced by replay payloads.
pub(crate) fn collect_replay_named_types(
    domain: &str,
    bindings: &ModuleBindings,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let codegen = ModuleCodegen::new(domain);

    for entry in bindings.values() {
        let CatalogEffect::External {
            replay: CatalogReplayPolicy::Recordable,
        } = entry.effect
        else {
            continue;
        };
        if entry.replay_kind != CatalogBindingReplayKind::BindingCall {
            continue;
        }

        if matches!(
            entry.replay_payload,
            CatalogReplayPayload::ArgumentsAndResults
        ) {
            for param in &entry.parameters {
                codegen.collect_replay_type_names(&param.binding_type, &mut names);
            }
        }
        codegen.collect_replay_type_names(&entry.return_binding, &mut names);
    }

    names
}

/// Collect VM-visible named types referenced by replay collection decoding.
pub(crate) fn collect_replay_vm_named_types(
    domain: &str,
    bindings: &ModuleBindings,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let codegen = ModuleCodegen::new(domain);
    for entry in bindings.values() {
        let CatalogEffect::External {
            replay: CatalogReplayPolicy::Recordable,
        } = entry.effect
        else {
            continue;
        };
        if entry.replay_kind != CatalogBindingReplayKind::BindingCall {
            continue;
        }

        if matches!(
            entry.replay_payload,
            CatalogReplayPayload::ArgumentsAndResults
        ) {
            for param in &entry.parameters {
                codegen.collect_replay_vm_type_names(&param.binding_type, &mut names);
            }
        }
        codegen.collect_replay_vm_type_names(&entry.return_binding, &mut names);
    }

    names
}

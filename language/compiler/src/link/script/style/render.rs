use std::collections::{BTreeMap, BTreeSet};

use destack_artifact::{OutputContent, OutputFile};
use destack_css as css;
use destack_source::{FileType, ModuleId, Uri};
use indexmap::{IndexMap, IndexSet};

use crate::{LinkError, LinkResult};

use super::super::plan::Plan;
use super::super::{AssetReference, ScriptLinker};
use crate::link::{OutputLocation, TargetLocation};

/// Push one rendered body fragment in source order.
fn push_rendered_css_body(body: &mut String, source: &str, is_minified: bool) {
    if source.is_empty() {
        return;
    }

    if !body.is_empty() && !is_minified {
        body.push('\n');
    }

    body.push_str(source);
}

/// Return one final stylesheet string from hoisted imports and body rules.
fn render_stylesheet_text(
    import_rules: IndexSet<String>,
    body: String,
    is_minified: bool,
) -> String {
    let mut rendered = String::new();

    for import_rule in import_rules {
        if !rendered.is_empty() && !is_minified {
            rendered.push('\n');
        }

        rendered.push_str(&import_rule);
    }

    if !body.is_empty() {
        if !rendered.is_empty() && !is_minified {
            rendered.push('\n');
        }

        rendered.push_str(&body);
    }

    rendered
}

impl<'a> ScriptLinker<'a> {
    /// Render the emitted stylesheet outputs for one output plan.
    pub(crate) fn render_css_stylesheet_outputs(&self, plan: &Plan) -> LinkResult<Vec<OutputFile>> {
        let mut files = Vec::new();

        // render one stylesheet bundle per entry module
        for module_id in plan.stylesheet_module_ids() {
            let rendered = self.render_stylesheet_output(module_id, plan)?;
            let output_location =
                plan.stylesheet_output_location(module_id)
                    .ok_or_else(|| LinkError::Internal {
                        anchor: (self.package_id).into(),
                        package: self.package_id,
                        message: format!(
                            "missing planned output location for stylesheet {:?}",
                            module_id
                        ),
                    })?;
            let module = self.module(module_id);

            files.push(OutputFile {
                uri: Uri::from_path(output_location.path()),
                content: OutputContent::Text {
                    code: rendered,
                    file_type: FileType::Css,
                },
                source: Some(module.uri.clone()),
            });
        }

        Ok(files)
    }

    /// Render one final bundled stylesheet output from one entry stylesheet module.
    fn render_stylesheet_output(&self, module_id: ModuleId, plan: &Plan) -> LinkResult<String> {
        let stylesheet_location =
            plan.stylesheet_output_location(module_id)
                .ok_or_else(|| LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!(
                        "missing planned output location for stylesheet {:?}",
                        module_id
                    ),
                })?;
        let target_location = TargetLocation::new(self.package_dir, self.target);
        let mut stack = Vec::new();
        let mut context = Vec::new();
        let mut emitted_modules = BTreeSet::new();
        let mut next_rewrite_index = 0;
        let (import_rules, body) = self.render_module_stylesheet(
            module_id,
            &mut stack,
            &mut context,
            &mut emitted_modules,
            &mut next_rewrite_index,
            stylesheet_location,
            &target_location,
            plan.asset_reference_map(),
        )?;
        let mut rendered = render_stylesheet_text(import_rules, body, self.should_compact_css());

        // keep text outputs newline terminated
        if !rendered.is_empty() && !rendered.ends_with('\n') {
            rendered.push('\n');
        }

        Ok(rendered)
    }

    /// Render one stylesheet module into final CSS source.
    #[allow(clippy::too_many_arguments)]
    fn render_module_stylesheet(
        &self,
        module_id: ModuleId,
        stack: &mut Vec<ModuleId>,
        context: &mut Vec<String>,
        emitted_modules: &mut BTreeSet<(ModuleId, Vec<String>)>,
        next_rewrite_index: &mut usize,
        stylesheet_location: &OutputLocation,
        target_location: &TargetLocation<'_>,
        assets: &IndexMap<ModuleId, AssetReference>,
    ) -> LinkResult<(IndexSet<String>, String)> {
        let module = self.module(module_id);
        let module = module.as_ref();
        let module_edges = self.module_edges_for_module(module_id)?;

        // emit each reachable css module once per effective import context
        if !emitted_modules.insert((module_id, context.clone())) {
            return Ok((IndexSet::new(), String::new()));
        }

        // reject css import cycles explicitly
        if stack.contains(&module_id) {
            return Err(LinkError::InvalidTarget {
                anchor: module_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                message: format!("css import cycle detected at '{}'", module.uri),
            });
        }

        stack.push(module_id);

        let mut css = self.css_payload(module_id)?;
        let mut rewrites = IndexMap::new();
        let mut import_targets = BTreeMap::new();

        self.plan_stylesheet_rewrites(
            module,
            &module_edges,
            &mut css,
            &mut import_targets,
            &mut rewrites,
            next_rewrite_index,
        )?;

        self.rewrite_stylesheet(
            &mut css.tree,
            css.stylesheet,
            &rewrites,
            stylesheet_location,
            target_location,
            assets,
        )?;

        let mut import_rules = IndexSet::new();
        let mut body = String::new();
        let is_minified = self.should_compact_css();
        let rule_ids = css.tree.get(css.stylesheet).rules.clone();

        // top level rules
        for rule_id in rule_ids {
            let Some(imported_module_id) = import_targets.get(&rule_id.id).copied().flatten()
            else {
                let rule = css::print::print_rule(&css.tree, rule_id);

                if matches!(css.tree.get(rule_id), css::Rule::Import(_)) {
                    import_rules.insert(rule);
                } else {
                    push_rendered_css_body(&mut body, &rule, is_minified);
                }

                continue;
            };
            let import_rule = match css.tree.get(rule_id) {
                css::Rule::Import(import_rule) => import_rule,
                _ => {
                    return Err(LinkError::Internal {
                        anchor: (self.package_id).into(),
                        package: self.package_id,
                        message: "inlined css import did not point to one import rule".to_string(),
                    });
                }
            };
            let context_len = context.len();

            if let Some(import_context) = self.css_import_context_signature(&css.tree, import_rule)
            {
                context.push(import_context);
            }

            let (imported_import_rules, imported_body) = self.render_module_stylesheet(
                imported_module_id,
                stack,
                context,
                emitted_modules,
                next_rewrite_index,
                stylesheet_location,
                target_location,
                assets,
            )?;

            context.truncate(context_len);

            if self.css_import_has_wrappers(import_rule) {
                let wrapped = self.wrap_css_import_stylesheet(
                    &css.tree,
                    import_rule,
                    render_stylesheet_text(imported_import_rules, imported_body, is_minified),
                )?;

                push_rendered_css_body(&mut body, &wrapped, is_minified);
            } else {
                for import_rule in imported_import_rules {
                    import_rules.insert(import_rule);
                }

                push_rendered_css_body(&mut body, &imported_body, is_minified);
            }
        }

        stack.pop();

        Ok((import_rules, body))
    }

    /// Rewrite final linked references in one stylesheet tree.
    fn rewrite_stylesheet(
        &self,
        tree: &mut css::Tree,
        stylesheet_id: css::LocalNodeId<css::Stylesheet>,
        rewrites: &IndexMap<String, (ModuleId, String)>,
        stylesheet_location: &OutputLocation,
        target_location: &TargetLocation<'_>,
        assets: &IndexMap<ModuleId, AssetReference>,
    ) -> LinkResult<()> {
        let replacements =
            self.stylesheet_replacements(rewrites, stylesheet_location, target_location, assets)?;
        let rule_ids = tree.get(stylesheet_id).rules.clone();

        for rule_id in rule_ids {
            self.rewrite_rule(tree, rule_id, &replacements)?;
        }

        Ok(())
    }

    /// Resolve the final replacement strings for one stylesheet rewrite set.
    fn stylesheet_replacements(
        &self,
        rewrites: &IndexMap<String, (ModuleId, String)>,
        stylesheet_location: &OutputLocation,
        target_location: &TargetLocation<'_>,
        assets: &IndexMap<ModuleId, AssetReference>,
    ) -> LinkResult<IndexMap<String, String>> {
        let mut replacements = IndexMap::new();

        for (placeholder, (module_id, suffix)) in rewrites {
            let Some(asset_reference) = assets.get(module_id) else {
                return Err(LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!(
                        "missing planned output for css asset '{}'",
                        self.asset_display_name(*module_id)
                    ),
                });
            };
            let replacement = match asset_reference {
                AssetReference::Inline { url } => url.clone(),
                AssetReference::Emitted { output_location } => {
                    let mut reference =
                        target_location.runtime_reference(stylesheet_location, output_location);

                    reference.push_str(suffix);
                    reference
                }
                AssetReference::Original => self.asset_original_reference(*module_id),
            };

            replacements.insert(placeholder.clone(), replacement);
        }

        Ok(replacements)
    }

    /// Rewrite one CSS rule subtree in place.
    fn rewrite_rule(
        &self,
        tree: &mut css::Tree,
        rule_id: css::LocalNodeId<css::Rule>,
        replacements: &IndexMap<String, String>,
    ) -> LinkResult<()> {
        let mut declaration_block = None;
        let mut nested_rules = Vec::new();
        let mut page_margin_rules = Vec::new();
        let mut supports_condition = None;

        match tree.get_mut(rule_id) {
            css::Rule::Import(rule) => {
                supports_condition = rule.supports;
            }
            css::Rule::Style(rule) => {
                declaration_block = rule.declarations;
                nested_rules = rule.rules.clone();
            }
            css::Rule::Nesting(rule) => {
                declaration_block = rule.declarations;
                nested_rules = rule.rules.clone();
            }
            css::Rule::Media(rule) => {
                nested_rules = rule.rules.clone();
            }
            css::Rule::Supports(rule) => {
                supports_condition = Some(rule.condition);
                nested_rules = rule.rules.clone();
            }
            css::Rule::LayerBlock(rule) => {
                nested_rules = rule.rules.clone();
            }
            css::Rule::Container(rule) => {
                nested_rules = rule.rules.clone();
            }
            css::Rule::Scope(rule) => {
                nested_rules = rule.rules.clone();
            }
            css::Rule::StartingStyle(rule) => {
                nested_rules = rule.rules.clone();
            }
            css::Rule::Keyframes(rule) => {
                nested_rules = rule.rules.clone();
            }
            css::Rule::MozDocument(rule) => {
                nested_rules = rule.rules.clone();
            }
            css::Rule::LayerStatement(_) => {}
            css::Rule::FontFeatureValues(_) => {}
            css::Rule::Namespace(_) => {}
            css::Rule::CustomMedia(_) => {}
            css::Rule::Property(rule) => {
                if let Some(initial_value) = &mut rule.initial_value {
                    self.rewrite_component_values(initial_value.components_mut(), replacements);
                }
            }
            css::Rule::Unknown(rule) => {
                self.rewrite_component_values(&mut rule.prelude, replacements);

                if let Some(block) = &mut rule.block {
                    self.rewrite_component_values(block, replacements);
                }
            }
            css::Rule::Custom(rule) => {
                self.rewrite_component_values(&mut rule.components, replacements);
            }
            css::Rule::Page(rule) => {
                declaration_block = rule.declarations;
                page_margin_rules = rule.page_margin_rules.clone();
            }
            css::Rule::FontFace(rule) => {
                declaration_block = rule.declarations;
            }
            css::Rule::FontPaletteValues(rule) => {
                declaration_block = rule.declarations;
            }
            css::Rule::CounterStyle(rule) => {
                declaration_block = rule.declarations;
            }
            css::Rule::Viewport(rule) => {
                declaration_block = rule.declarations;
            }
            css::Rule::ViewTransition(rule) => {
                declaration_block = rule.declarations;
            }
            css::Rule::NestedDeclarations(rule) => {
                declaration_block = rule.declarations;
            }
            css::Rule::Keyframe(rule) => {
                declaration_block = rule.declarations;
            }
            css::Rule::Ignored(_) => {}
        }

        if let Some(supports_condition) = supports_condition {
            self.rewrite_supports_condition(tree, supports_condition, replacements);
        }

        if let Some(declaration_block) = declaration_block {
            self.rewrite_declaration_block(tree, declaration_block, replacements)?;
        }

        for page_margin_rule in page_margin_rules {
            self.rewrite_page_margin_rule(tree, page_margin_rule, replacements)?;
        }

        for nested_rule in nested_rules {
            self.rewrite_rule(tree, nested_rule, replacements)?;
        }

        Ok(())
    }

    /// Rewrite one page margin rule subtree in place.
    fn rewrite_page_margin_rule(
        &self,
        tree: &mut css::Tree,
        rule_id: css::LocalNodeId<css::PageMarginRule>,
        replacements: &IndexMap<String, String>,
    ) -> LinkResult<()> {
        let declaration_block = {
            let rule = tree.get_mut(rule_id);
            rule.declarations
        };

        if let Some(declaration_block) = declaration_block {
            self.rewrite_declaration_block(tree, declaration_block, replacements)?;
        }

        Ok(())
    }

    /// Rewrite one declaration block subtree in place.
    fn rewrite_declaration_block(
        &self,
        tree: &mut css::Tree,
        declaration_block_id: css::LocalNodeId<css::DeclarationBlock>,
        replacements: &IndexMap<String, String>,
    ) -> LinkResult<()> {
        let declaration_ids = tree.get(declaration_block_id).declarations.clone();

        for declaration_id in declaration_ids {
            let declaration = tree.get_mut(declaration_id);
            self.rewrite_component_values(declaration.value.components_mut(), replacements);
        }

        Ok(())
    }

    /// Rewrite one component value list in place.
    fn rewrite_component_values(
        &self,
        components: &mut css::ComponentValueList,
        replacements: &IndexMap<String, String>,
    ) {
        for value in &mut components.values {
            match value {
                css::ComponentValue::Token(css::Token::String(text))
                | css::ComponentValue::Token(css::Token::UnquotedUrl { value: text, .. }) => {
                    if let Some(replacement) = replacements.get(text) {
                        *text = replacement.clone();
                    }
                }
                css::ComponentValue::Function(css::Function { arguments, .. }) => {
                    self.rewrite_component_values(arguments, replacements);
                }
                css::ComponentValue::Block(block) => {
                    self.rewrite_component_values(&mut block.value, replacements);
                }
                css::ComponentValue::Token(_) => {}
            }
        }
    }

    /// Rewrite one supports condition in place.
    fn rewrite_supports_condition(
        &self,
        tree: &mut css::Tree,
        condition_id: css::LocalNodeId<css::SupportsCondition>,
        replacements: &IndexMap<String, String>,
    ) {
        let condition = tree.get(condition_id).clone();

        match condition {
            css::SupportsCondition::Not(condition) => {
                self.rewrite_supports_condition(tree, condition, replacements);
            }
            css::SupportsCondition::And(conditions) | css::SupportsCondition::Or(conditions) => {
                for condition in conditions {
                    self.rewrite_supports_condition(tree, condition, replacements);
                }
            }
            css::SupportsCondition::Declaration { .. } => {
                let condition = tree.get_mut(condition_id);
                let css::SupportsCondition::Declaration { value, .. } = condition else {
                    unreachable!();
                };

                self.rewrite_component_values(value.components_mut(), replacements);
            }
            css::SupportsCondition::Selector(_) | css::SupportsCondition::Unknown(_) => {}
        }
    }

    /// Return whether linked CSS output should use compact assembly.
    pub(super) fn should_compact_css(&self) -> bool {
        self.target.should_minify_bundle_css_whitespace()
            || self.target.should_minify_bundle_css_syntax()
    }

    /// Wrap one inlined stylesheet source for one conditioned import rule.
    pub(super) fn wrap_css_import_stylesheet(
        &self,
        tree: &css::Tree,
        import_rule: &css::ImportRule,
        mut source: String,
    ) -> LinkResult<String> {
        if let Some(media) = &import_rule.media {
            let media = css::print::print_media_query_list(tree, *media);
            source = format!("@media {media}{{{source}}}");
        }

        if let Some(supports_condition) = &import_rule.supports {
            let supports = css::print::print_supports_condition(tree, *supports_condition);
            source = format!("@supports {supports}{{{source}}}");
        }

        if let Some(layer) = &import_rule.layer {
            source = match &layer.name {
                Some(name) => format!(
                    "@layer {}{{{source}}}",
                    css::print::print_layer_name_list(name)
                ),
                None => format!("@layer{{{source}}}"),
            };
        }

        if !self.should_compact_css() && !source.ends_with('\n') {
            source.push('\n');
        }

        Ok(source)
    }
}

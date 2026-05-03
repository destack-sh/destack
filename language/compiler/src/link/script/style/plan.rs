use std::collections::{BTreeMap, BTreeSet};

use css::print::{print_layer_name_list, print_media_query_list, print_supports_condition};
use css::{
    ComponentValue, ComponentValueList, DeclarationBlock, Function, ImportResource, ImportRule,
    LocalNodeId, Rule, SupportsCondition, Token, Tree, UrlResource,
};
use destack_artifact::{Css, Data};
use destack_css as css;
use destack_source::{FileType, ModuleEdge, ModuleEdgeRelation, ModuleId, StringId};
use destack_workspace::{Module, ProviderError};
use indexmap::{IndexMap, IndexSet};

use crate::link::OutputLocation;
use crate::{CompilerResult, LinkError, LinkResult};

use super::super::ScriptLinker;
use crate::CompilerError;

impl<'a> ScriptLinker<'a> {
    /// Collect the rooted stylesheet module ids for one target entry.
    pub(in crate::link::script) fn collect_stylesheet_modules(
        &self,
        stylesheet_root_modules: &[ModuleId],
        script_module_ids: &[ModuleId],
    ) -> Vec<ModuleId> {
        let mut stylesheet_module_ids = stylesheet_root_modules
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();

        // script graphs root css modules through JS import edges
        for module_id in script_module_ids {
            let module = self.module(*module_id);

            let file = self.file(module.file_id);

            if file.ty == FileType::Css {
                stylesheet_module_ids.insert(*module_id);
            }
        }

        stylesheet_module_ids.into_iter().collect()
    }

    /// Require the artifacts needed for one rooted stylesheet lane.
    pub(in crate::link::script) fn require_stylesheet_artifacts(
        &self,
        root_module_ids: &[ModuleId],
    ) -> CompilerResult<()> {
        let mut blocked = Vec::new();
        let mut queued_module_ids = root_module_ids.to_vec();
        let mut visited_module_ids = BTreeSet::new();

        // stylesheet planning needs every css module in the reachable import closure
        while let Some(module_id) = queued_module_ids.pop() {
            if !visited_module_ids.insert(module_id) {
                continue;
            }

            let profile_id = self.profile_id_for_module(module_id)?;
            let module = self.module(module_id);

            let file = self.file(module.file_id);

            if file.ty != FileType::Css {
                return Err(LinkError::InvalidModuleKind {
                    anchor: module_id.into(),
                    package: self.package_id,
                    target: self.target_id.clone(),
                    subject: "stylesheet module".to_string(),
                    expected: "css module".to_string(),
                    found: module.uri.to_string(),
                }
                .into());
            }

            match self
                .compiler
                .require_dir_exported(self.context, module_id, profile_id)
            {
                Ok(_) => {}
                Err(ProviderError::Blocked { keys }) => blocked.extend(keys),
                Err(error) => return Err(CompilerError::from(error)),
            }

            let module_edges = self.module_edges_for_module(module_id)?;

            for edge in module_edges
                .iter()
                .filter(|edge| edge.relation == ModuleEdgeRelation::StyleImport)
            {
                queued_module_ids.push(edge.target);
            }
        }

        if !blocked.is_empty() {
            return Err(CompilerError::Blocked { keys: blocked });
        }

        Ok(())
    }

    /// Collect the asset module ids referenced by one stylesheet lane.
    pub(in crate::link::script) fn collect_stylesheet_assets(
        &self,
        stylesheet_module_ids: &[ModuleId],
    ) -> CompilerResult<IndexSet<ModuleId>> {
        let mut asset_module_ids = IndexSet::new();
        let mut queued_module_ids = stylesheet_module_ids.to_vec();
        let mut visited_module_ids = BTreeSet::new();

        if stylesheet_module_ids.is_empty() {
            return Ok(asset_module_ids);
        }

        self.require_stylesheet_artifacts(stylesheet_module_ids)?;

        while let Some(module_id) = queued_module_ids.pop() {
            if !visited_module_ids.insert(module_id) {
                continue;
            }

            let module_edges = self
                .module_edges_for_module(module_id)
                .map_err(CompilerError::from)?;

            for edge in module_edges
                .iter()
                .filter(|edge| edge.relation == ModuleEdgeRelation::StyleImport)
            {
                queued_module_ids.push(edge.target);
            }

            for edge in module_edges
                .iter()
                .filter(|edge| edge.relation == ModuleEdgeRelation::StyleUrl)
            {
                asset_module_ids.insert(edge.target);
            }
        }

        Ok(asset_module_ids)
    }

    /// Plan one emitted output location for each stylesheet module.
    pub(crate) fn plan_css_stylesheet_outputs(
        &self,
        stylesheet_module_ids: &[ModuleId],
    ) -> LinkResult<IndexMap<ModuleId, OutputLocation>> {
        let mut output_locations = IndexMap::new();

        // derive one output file path per entry stylesheet module
        for module_id in stylesheet_module_ids.iter().copied() {
            let module = self.module(module_id);
            let output_location = self.css_stylesheet_output_location(module.as_ref())?;

            output_locations.insert(module_id, output_location);
        }

        Ok(output_locations)
    }

    /// Return the CSS payload for one parsed module.
    pub(super) fn css_payload(&self, module_id: ModuleId) -> LinkResult<Css> {
        let data = self.data(module_id)?;
        let module = self.module(module_id);

        match data.as_ref() {
            Data::Css(css) => Ok(css.clone()),
            Data::Json(_) | Data::Html(_) => Err(LinkError::InvalidModuleKind {
                anchor: module_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                subject: "stylesheet module".to_string(),
                expected: "css module".to_string(),
                found: module.uri.to_string(),
            }
            .into()),
        }
    }

    /// Return whether one CSS import rule carries wrapper conditions.
    pub(super) fn css_import_has_wrappers(&self, import_rule: &ImportRule) -> bool {
        import_rule.media.is_some() || import_rule.supports.is_some() || import_rule.layer.is_some()
    }

    /// Return one stable import-context signature when this import wraps its target.
    pub(super) fn css_import_context_signature(
        &self,
        tree: &Tree,
        import_rule: &ImportRule,
    ) -> Option<String> {
        if !self.css_import_has_wrappers(import_rule) {
            return None;
        }

        let mut parts = Vec::new();

        if let Some(layer) = &import_rule.layer {
            let layer = match &layer.name {
                Some(name) => format!("layer={}", print_layer_name_list(name)),
                None => "layer".to_string(),
            };

            parts.push(layer);
        }

        if let Some(supports_condition) = &import_rule.supports {
            parts.push(format!(
                "supports={}",
                print_supports_condition(tree, *supports_condition)
            ));
        }

        if let Some(media) = &import_rule.media {
            parts.push(format!("media={}", print_media_query_list(tree, *media)));
        }

        Some(parts.join("|"))
    }

    /// Rewrite one CSS module tree in place and collect asset placeholder rewrites.
    pub(super) fn plan_stylesheet_rewrites(
        &self,
        module: &Module,
        module_edges: &[ModuleEdge],
        css: &mut Css,
        import_targets: &mut BTreeMap<u32, Option<ModuleId>>,
        rewrites: &mut IndexMap<String, (ModuleId, String)>,
        next_rewrite_index: &mut usize,
    ) -> LinkResult<()> {
        let rule_ids = css.tree.get(css.stylesheet).rules.clone();

        for rule_id in rule_ids {
            self.plan_css_rule_rewrites(
                module,
                module_edges,
                &mut css.tree,
                rule_id,
                import_targets,
                rewrites,
                next_rewrite_index,
            )?;
        }

        Ok(())
    }

    /// Rewrite one CSS rule subtree in place.
    fn plan_css_rule_rewrites(
        &self,
        module: &Module,
        module_edges: &[ModuleEdge],
        tree: &mut Tree,
        rule_id: LocalNodeId<Rule>,
        import_targets: &mut BTreeMap<u32, Option<ModuleId>>,
        rewrites: &mut IndexMap<String, (ModuleId, String)>,
        next_rewrite_index: &mut usize,
    ) -> LinkResult<()> {
        let mut declaration_block = None;
        let mut nested_rules = Vec::new();
        let mut page_margin_rules = Vec::new();
        let mut supports_condition = None;

        // current rule
        match tree.get(rule_id).clone() {
            Rule::Import(rule) => {
                let import_target = self.resolve_css_import_target(
                    module,
                    module_edges,
                    rule.resource.as_ref(),
                    &rule.url,
                )?;
                import_targets.insert(rule_id.id, import_target);
                supports_condition = rule.supports;
            }
            Rule::Style(rule) => {
                declaration_block = rule.declarations;
                nested_rules = rule.rules.clone();
            }
            Rule::Nesting(rule) => {
                declaration_block = rule.declarations;
                nested_rules = rule.rules.clone();
            }
            Rule::Media(rule) => {
                nested_rules = rule.rules.clone();
            }
            Rule::Supports(rule) => {
                supports_condition = Some(rule.condition);
                nested_rules = rule.rules.clone();
            }
            Rule::LayerBlock(rule) => {
                nested_rules = rule.rules.clone();
            }
            Rule::Container(rule) => {
                nested_rules = rule.rules.clone();
            }
            Rule::Scope(rule) => {
                nested_rules = rule.rules.clone();
            }
            Rule::StartingStyle(rule) => {
                nested_rules = rule.rules.clone();
            }
            Rule::Keyframes(rule) => {
                nested_rules = rule.rules.clone();
            }
            Rule::MozDocument(rule) => {
                nested_rules = rule.rules.clone();
            }
            Rule::LayerStatement(_) => {}
            Rule::FontFeatureValues(_) => {}
            Rule::Namespace(_) => {}
            Rule::CustomMedia(_) => {}
            Rule::Property(rule) => {
                if let Some(mut initial_value) = rule.initial_value {
                    self.plan_css_component_value_rewrites(
                        module,
                        module_edges,
                        tree,
                        initial_value.components_mut(),
                        rewrites,
                        next_rewrite_index,
                    )?;

                    let Rule::Property(rule) = tree.get_mut(rule_id) else {
                        return Err(LinkError::Internal {
                            anchor: (self.package_id).into(),
                            package: self.package_id,
                            message: format!(
                                "property rule changed shape during stylesheet planning: {:?}",
                                rule_id
                            ),
                        }
                        .into());
                    };
                    rule.initial_value = Some(initial_value);
                }
            }
            Rule::Unknown(mut rule) => {
                self.plan_css_component_value_rewrites(
                    module,
                    module_edges,
                    tree,
                    &mut rule.prelude,
                    rewrites,
                    next_rewrite_index,
                )?;

                if let Some(block) = &mut rule.block {
                    self.plan_css_component_value_rewrites(
                        module,
                        module_edges,
                        tree,
                        block,
                        rewrites,
                        next_rewrite_index,
                    )?;
                }

                let Rule::Unknown(current_rule) = tree.get_mut(rule_id) else {
                    return Err(LinkError::Internal {
                        anchor: (self.package_id).into(),
                        package: self.package_id,
                        message: format!(
                            "unknown rule changed shape during stylesheet planning: {:?}",
                            rule_id
                        ),
                    }
                    .into());
                };
                current_rule.prelude = rule.prelude;
                current_rule.block = rule.block;
            }
            Rule::Custom(mut rule) => {
                self.plan_css_component_value_rewrites(
                    module,
                    module_edges,
                    tree,
                    &mut rule.components,
                    rewrites,
                    next_rewrite_index,
                )?;

                let Rule::Custom(current_rule) = tree.get_mut(rule_id) else {
                    return Err(LinkError::Internal {
                        anchor: (self.package_id).into(),
                        package: self.package_id,
                        message: format!(
                            "custom rule changed shape during stylesheet planning: {:?}",
                            rule_id
                        ),
                    }
                    .into());
                };
                current_rule.components = rule.components;
            }
            Rule::Page(rule) => {
                declaration_block = rule.declarations;
                page_margin_rules = rule.page_margin_rules.clone();
            }
            Rule::FontFace(rule) => {
                declaration_block = rule.declarations;
            }
            Rule::FontPaletteValues(rule) => {
                declaration_block = rule.declarations;
            }
            Rule::CounterStyle(rule) => {
                declaration_block = rule.declarations;
            }
            Rule::Viewport(rule) => {
                declaration_block = rule.declarations;
            }
            Rule::ViewTransition(rule) => {
                declaration_block = rule.declarations;
            }
            Rule::NestedDeclarations(rule) => {
                declaration_block = rule.declarations;
            }
            Rule::Keyframe(rule) => {
                declaration_block = rule.declarations;
            }
            Rule::Ignored(_) => {}
        }

        // supports
        if let Some(supports_condition) = supports_condition {
            self.plan_css_supports_condition_rewrites(
                module,
                module_edges,
                tree,
                supports_condition,
                rewrites,
                next_rewrite_index,
            )?;
        }

        // declarations
        if let Some(declaration_block) = declaration_block {
            self.plan_css_declaration_block_rewrites(
                module,
                module_edges,
                tree,
                declaration_block,
                rewrites,
                next_rewrite_index,
            )?;
        }

        // nested page margin rules
        for page_margin_rule in page_margin_rules {
            self.plan_css_page_margin_rule_rewrites(
                module,
                module_edges,
                tree,
                page_margin_rule,
                rewrites,
                next_rewrite_index,
            )?;
        }

        // nested rules
        for nested_rule in nested_rules {
            self.plan_css_rule_rewrites(
                module,
                module_edges,
                tree,
                nested_rule,
                import_targets,
                rewrites,
                next_rewrite_index,
            )?;
        }

        Ok(())
    }

    /// Rewrite one page margin rule subtree in place.
    fn plan_css_page_margin_rule_rewrites(
        &self,
        module: &Module,
        module_edges: &[ModuleEdge],
        tree: &mut Tree,
        rule_id: LocalNodeId<css::PageMarginRule>,
        rewrites: &mut IndexMap<String, (ModuleId, String)>,
        next_rewrite_index: &mut usize,
    ) -> LinkResult<()> {
        let declaration_block = {
            let rule = tree.get_mut(rule_id);
            rule.declarations
        };

        if let Some(declaration_block) = declaration_block {
            self.plan_css_declaration_block_rewrites(
                module,
                module_edges,
                tree,
                declaration_block,
                rewrites,
                next_rewrite_index,
            )?;
        }

        Ok(())
    }

    /// Rewrite one declaration block subtree in place.
    fn plan_css_declaration_block_rewrites(
        &self,
        module: &Module,
        module_edges: &[ModuleEdge],
        tree: &mut Tree,
        declaration_block_id: LocalNodeId<DeclarationBlock>,
        rewrites: &mut IndexMap<String, (ModuleId, String)>,
        next_rewrite_index: &mut usize,
    ) -> LinkResult<()> {
        let declaration_ids = tree.get(declaration_block_id).declarations.clone();

        for declaration_id in declaration_ids {
            let mut components = tree.get(declaration_id).value.components().clone();
            self.plan_css_component_value_rewrites(
                module,
                module_edges,
                tree,
                &mut components,
                rewrites,
                next_rewrite_index,
            )?;

            let declaration = tree.get_mut(declaration_id);
            declaration.value.components = components;
        }

        Ok(())
    }

    /// Rewrite one component value list in place.
    fn plan_css_component_value_rewrites(
        &self,
        module: &Module,
        module_edges: &[ModuleEdge],
        tree: &Tree,
        components: &mut ComponentValueList,
        rewrites: &mut IndexMap<String, (ModuleId, String)>,
        next_rewrite_index: &mut usize,
    ) -> LinkResult<()> {
        for value in &mut components.values {
            match value {
                ComponentValue::Token(Token::UnquotedUrl {
                    value: url,
                    url_resource,
                }) => {
                    if let Some(rewrite) = self.plan_css_url_rewrite(
                        module,
                        module_edges,
                        url_resource.as_ref(),
                        url,
                        next_rewrite_index,
                    )? {
                        let (placeholder, module_id, suffix) = rewrite;

                        *url = placeholder.clone();
                        rewrites.insert(placeholder, (module_id, suffix));
                    }
                }
                ComponentValue::Function(function) if function.name_eq(&tree.strings, "url") => {
                    self.plan_css_url_function_rewrite(
                        module,
                        module_edges,
                        tree,
                        function,
                        rewrites,
                        next_rewrite_index,
                    )?;
                }
                ComponentValue::Function(function) => {
                    self.plan_css_component_value_rewrites(
                        module,
                        module_edges,
                        tree,
                        &mut function.arguments,
                        rewrites,
                        next_rewrite_index,
                    )?;
                }
                ComponentValue::Block(block) => {
                    self.plan_css_component_value_rewrites(
                        module,
                        module_edges,
                        tree,
                        &mut block.value,
                        rewrites,
                        next_rewrite_index,
                    )?;
                }
                ComponentValue::Token(_) => {}
            }
        }

        Ok(())
    }

    /// Rewrite one supports condition in place.
    fn plan_css_supports_condition_rewrites(
        &self,
        module: &Module,
        module_edges: &[ModuleEdge],
        tree: &mut Tree,
        condition_id: LocalNodeId<SupportsCondition>,
        rewrites: &mut IndexMap<String, (ModuleId, String)>,
        next_rewrite_index: &mut usize,
    ) -> LinkResult<()> {
        let condition = tree.get(condition_id).clone();

        match condition {
            SupportsCondition::Not(condition) => self.plan_css_supports_condition_rewrites(
                module,
                module_edges,
                tree,
                condition,
                rewrites,
                next_rewrite_index,
            )?,
            SupportsCondition::And(conditions) | SupportsCondition::Or(conditions) => {
                for condition in conditions {
                    self.plan_css_supports_condition_rewrites(
                        module,
                        module_edges,
                        tree,
                        condition,
                        rewrites,
                        next_rewrite_index,
                    )?;
                }
            }
            SupportsCondition::Declaration { .. } => {
                let SupportsCondition::Declaration { value, .. } = tree.get(condition_id) else {
                    return Err(LinkError::Internal {
                        anchor: (self.package_id).into(),
                        package: self.package_id,
                        message: format!(
                            "supports declaration changed shape during stylesheet planning: {:?}",
                            condition_id
                        ),
                    }
                    .into());
                };
                let mut components = value.components().clone();

                self.plan_css_component_value_rewrites(
                    module,
                    module_edges,
                    tree,
                    &mut components,
                    rewrites,
                    next_rewrite_index,
                )?;

                let condition = tree.get_mut(condition_id);
                let SupportsCondition::Declaration { value, .. } = condition else {
                    return Err(LinkError::Internal {
                        anchor: (self.package_id).into(),
                        package: self.package_id,
                        message: format!(
                            "supports declaration changed shape during stylesheet rewrite: {:?}",
                            condition_id
                        ),
                    }
                    .into());
                };
                value.components = components;
            }
            SupportsCondition::Selector(_) | SupportsCondition::Unknown(_) => {}
        }

        Ok(())
    }

    /// Rewrite one `url(...)` function in place.
    fn plan_css_url_function_rewrite(
        &self,
        module: &Module,
        module_edges: &[ModuleEdge],
        tree: &Tree,
        function: &mut Function,
        rewrites: &mut IndexMap<String, (ModuleId, String)>,
        next_rewrite_index: &mut usize,
    ) -> LinkResult<()> {
        let url = self.css_function_url_value(tree, &function.arguments);
        let Some(resource) = function.url_resource.as_ref() else {
            return Ok(());
        };

        if resource.is_external {
            return Ok(());
        }

        for value in &mut function.arguments.values {
            match value {
                ComponentValue::Token(Token::String(_))
                | ComponentValue::Token(Token::UnquotedUrl { .. }) => {
                    let rewrite = self
                        .plan_css_asset_rewrite(
                            module,
                            module_edges,
                            resource,
                            &url,
                            next_rewrite_index,
                        )?
                        .ok_or_else(|| LinkError::Internal {
                            anchor: (self.package_id).into(),
                package: self.package_id,
                            message: format!(
                                "missing css url graph edge for module '{}' site {} and specifier '{}'",
                                module.uri, resource.id, url
                            ),
                        })?;

                    let (placeholder, module_id, suffix) = rewrite;

                    *value = ComponentValue::Token(Token::String(placeholder.clone()));
                    rewrites.insert(placeholder, (module_id, suffix));

                    return Ok(());
                }
                ComponentValue::Token(Token::WhiteSpace(_))
                | ComponentValue::Token(Token::Comment(_)) => {}
                ComponentValue::Function(_)
                | ComponentValue::Block(_)
                | ComponentValue::Token(_) => {
                    return Err(LinkError::InvalidTarget {
                        anchor: module.id.into(),
                        package: self.package_id,
                        target: self.target_id.clone(),
                        message: format!(
                            "css local url() rewrites require a direct string or unquoted url token: '{}'",
                            url
                        ),
                    }.into());
                }
            }
        }

        Err(LinkError::InvalidTarget {
            anchor: module.id.into(),
            package: self.package_id,
            target: self.target_id.clone(),
            message: format!(
                "css local url() rewrites require a direct string or unquoted url token: '{}'",
                url
            ),
        })
    }

    /// Plan one stylesheet rewrite for one asset URL rewrite site.
    fn plan_css_url_rewrite(
        &self,
        module: &Module,
        module_edges: &[ModuleEdge],
        resource: Option<&UrlResource>,
        url: &str,
        next_rewrite_index: &mut usize,
    ) -> LinkResult<Option<(String, ModuleId, String)>> {
        let Some(resource) = resource else {
            return Ok(None);
        };

        if resource.is_external {
            return Ok(None);
        };

        self.plan_css_asset_rewrite(module, module_edges, resource, url, next_rewrite_index)
    }

    /// Build one stylesheet rewrite for one local asset URL site.
    fn plan_css_asset_rewrite(
        &self,
        module: &Module,
        module_edges: &[ModuleEdge],
        resource: &UrlResource,
        specifier: &str,
        next_rewrite_index: &mut usize,
    ) -> LinkResult<Option<(String, ModuleId, String)>> {
        let Some(target_module_id) =
            self.resolve_css_url_target(module, module_edges, resource, specifier)?
        else {
            return Ok(None);
        };
        let placeholder = self.next_stylesheet_placeholder(next_rewrite_index);
        let asset_module = self.module(target_module_id);
        Ok(Some((
            placeholder,
            asset_module.id,
            resource.suffix.clone(),
        )))
    }

    /// Return one unique stylesheet placeholder.
    fn next_stylesheet_placeholder(&self, next_rewrite_index: &mut usize) -> String {
        let placeholder = format!("__DESTACK_CSS_{}__", *next_rewrite_index);
        *next_rewrite_index += 1;

        placeholder
    }

    /// Resolve one emitted stylesheet output location for one source module.
    pub(super) fn css_stylesheet_output_location(
        &self,
        module: &Module,
    ) -> LinkResult<OutputLocation> {
        let _source_path = module.path.as_ref().ok_or_else(|| LinkError::Internal {
            anchor: (self.package_id).into(),
            package: self.package_id,
            message: format!("css stylesheet '{}' has no filesystem path", module.uri),
        })?;

        self.asset_output_location(module.id)
    }

    /// Return one stylesheet import target for one CSS import resource.
    fn resolve_css_import_target(
        &self,
        _module: &Module,
        module_edges: &[ModuleEdge],
        resource: Option<&ImportResource>,
        specifier: &str,
    ) -> LinkResult<Option<ModuleId>> {
        let Some(resource) = resource else {
            return Ok(None);
        };

        if resource.is_external {
            return Ok(None);
        }

        let specifier_id = StringId::for_text(specifier);

        Ok(self
            .module_edge_for_site_specifier(
                module_edges,
                destack_source::ModuleEdgeRelation::StyleImport,
                resource.id,
                specifier_id,
            )
            .map(|edge| edge.target))
    }

    /// Return one asset URL target for one CSS url resource.
    fn resolve_css_url_target(
        &self,
        _module: &Module,
        module_edges: &[ModuleEdge],
        resource: &UrlResource,
        specifier: &str,
    ) -> LinkResult<Option<ModuleId>> {
        let specifier_id = StringId::for_text(specifier);

        Ok(self
            .module_edge_for_site_specifier(
                module_edges,
                destack_source::ModuleEdgeRelation::StyleUrl,
                resource.id,
                specifier_id,
            )
            .map(|edge| edge.target))
    }

    /// Return one canonical CSS url() value string from one argument list.
    fn css_function_url_value(&self, _tree: &Tree, arguments: &ComponentValueList) -> String {
        if arguments.values.len() == 1 {
            match &arguments.values[0] {
                ComponentValue::Token(Token::String(value))
                | ComponentValue::Token(Token::UnquotedUrl { value, .. }) => {
                    return value.clone();
                }
                _ => {}
            }
        }

        arguments
            .values
            .iter()
            .find_map(|value| match value {
                ComponentValue::Token(Token::String(value))
                | ComponentValue::Token(Token::UnquotedUrl { value, .. }) => Some(value.clone()),
                ComponentValue::Token(Token::WhiteSpace(_)) => None,
                _ => None,
            })
            .unwrap_or_default()
    }
}

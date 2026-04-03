use std::path::Path;

use crate::resolve::module::reference::resolve_source_reference_path;
use crate::{Compiler, CompilerContext, ResolveError, ResolveResult};
use destack_artifact::{
    ArtifactKey, ArtifactStamp, Css, Data, DirResolved, Html, Loader, ModuleEdge,
    ModuleEdgeRelation, ModuleGraph, ModuleKind,
};
use destack_core::StringId;
use destack_dir::{DependencyItem, ModuleTarget};
use destack_html::{AttributeResource, HtmlResourceKind};
use destack_source::{ModuleId, ModuleVersion};
use destack_workspace::{Module, ProfileId};
use {destack_css as css, destack_html as html};

impl Compiler {
    /// Build or load the module graph for one profile.
    pub(crate) fn process_module_graph(
        &self,
        profile_id: ProfileId,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::module_graph(profile_id);

        // reuse one persisted module graph image when available
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| compiler.load_module_graph_image(revision, profile_id),
                |store, version, payload| store.publish_module_graph(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        // start with an empty graph when nothing has been published yet
        if self.module_graph(profile_id).is_none() {
            let graph = ModuleGraph::new(profile_id);
            context.publish_artifact(artifact_key, graph.clone(), |store, version, payload| {
                store.publish_module_graph(version, payload)
            });
            context.store_artifact(&artifact_key, &graph, |compiler, _artifact_stamp, graph| {
                compiler.store_module_graph_image(revision, profile_id, graph)
            });
        }

        Ok(())
    }

    /// Update the module graph from one resolved DIR snapshot.
    pub(crate) fn update_module_graph_from_dir(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        artifact_stamp: ArtifactStamp,
        dir: &DirResolved,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<()> {
        let revision = context.revision();
        let module = context.module(module_id);
        let file = context.file(module.file_id);
        let module_kind = ModuleKind::from_file_type_and_loader(file.ty, module.loader);
        let module_version = ModuleVersion::new(artifact_stamp.0);
        let dependencies =
            self.collect_module_graph_edges(revision, module.as_ref(), profile_id, dir, context)?;

        // update the graph for this profile
        let artifact_key = ArtifactKey::module_graph(profile_id);
        let graph = self
            .module_graph(profile_id)
            .map(|graph| graph.as_ref().clone())
            .unwrap_or_else(|| ModuleGraph::new(profile_id));
        let mut graph = graph;
        graph.update_module(module_id, module_kind, module_version, dependencies);
        context.publish_artifact(artifact_key, graph.clone(), |store, version, payload| {
            store.publish_module_graph(version, payload)
        });
        context.store_artifact(&artifact_key, &graph, |compiler, _artifact_stamp, graph| {
            compiler.store_module_graph_image(revision, profile_id, graph)
        });

        Ok(())
    }

    /// Collect graph edges for one resolved module snapshot.
    fn collect_module_graph_edges(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<Vec<ModuleEdge>> {
        let file = context.file(module.file_id);
        let module_kind = ModuleKind::from_file_type_and_loader(file.ty, module.loader);

        if module_kind == ModuleKind::Html {
            return self.collect_html_module_graph_edges(revision, module, profile_id, dir);
        }

        if module_kind == ModuleKind::Css {
            return self.collect_css_module_graph_edges(revision, module, profile_id, dir);
        }

        let mut dependencies = Vec::new();

        // import edges
        for ((_, specifier, relation, loader_override), targets_for_kind) in
            dir.imported_modules.iter()
        {
            let specifier = Some(*specifier);
            let loader = *loader_override;

            if let Some(target) = targets_for_kind.value {
                self.collect_module_graph_target_edges(
                    &mut dependencies,
                    revision,
                    module.id,
                    profile_id,
                    *relation,
                    specifier,
                    loader,
                    target,
                )?;
            }

            if let Some(target) = targets_for_kind.ty {
                self.collect_module_graph_target_edges(
                    &mut dependencies,
                    revision,
                    module.id,
                    profile_id,
                    *relation,
                    specifier,
                    loader,
                    target,
                )?;
            }
        }

        // namespace exports
        for export in dir.namespace_exports.iter() {
            let specifier = self.namespace_export_module_edge_specifier(dir, *export);
            self.collect_module_graph_target_edges(
                &mut dependencies,
                revision,
                module.id,
                profile_id,
                ModuleEdgeRelation::NamespaceExport,
                specifier,
                None,
                export.module_id,
            )?;
        }

        Ok(dependencies)
    }

    /// Collect graph edges from one HTML module.
    fn collect_html_module_graph_edges(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
    ) -> ResolveResult<Vec<ModuleEdge>> {
        let html = self.html_module_payload(module.id, profile_id, dir)?;
        let mut dependencies = Vec::new();
        let children = html.tree.get(html.document).children.clone();
        self.collect_html_module_graph_edges_in_nodes(
            revision,
            module,
            profile_id,
            dir,
            &html,
            &children,
            &mut dependencies,
        )?;

        Ok(dependencies)
    }

    /// Collect graph edges from one CSS module.
    fn collect_css_module_graph_edges(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
    ) -> ResolveResult<Vec<ModuleEdge>> {
        let css = self.css_module_payload(module.id, profile_id, dir)?;
        let mut dependencies = Vec::new();
        let rule_ids = css.tree.get(css.stylesheet).rules.clone();

        for rule_id in rule_ids {
            self.collect_css_module_graph_edges_in_rule(
                revision,
                module,
                profile_id,
                dir,
                &css,
                rule_id,
                &mut dependencies,
            )?;
        }

        Ok(dependencies)
    }

    /// Collect graph edges from one HTML node list.
    fn collect_html_module_graph_edges_in_nodes(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        html: &Html,
        nodes: &[html::LocalNodeId<html::Content>],
        dependencies: &mut Vec<ModuleEdge>,
    ) -> ResolveResult<()> {
        for node_id in nodes {
            let html::Content::Element(element) = html.tree.get(*node_id) else {
                continue;
            };

            for attribute_id in &element.attributes {
                let attribute = html.tree.get(*attribute_id);
                let Some(value) = attribute.value.as_ref() else {
                    continue;
                };
                let Some(resource) = value.resource.as_ref() else {
                    continue;
                };

                match resource {
                    AttributeResource::Resource(resource) => {
                        if resource.is_external {
                            continue;
                        }

                        let dependency = match resource.kind {
                            HtmlResourceKind::ModuleScript => self
                                .resolve_html_module_graph_script_edge(
                                    revision,
                                    module,
                                    profile_id,
                                    dir,
                                    attribute_id.id,
                                    &value.value,
                                    &resource.path,
                                )?,
                            HtmlResourceKind::Stylesheet => self
                                .resolve_html_module_graph_stylesheet_edge(
                                    revision,
                                    module,
                                    profile_id,
                                    dir,
                                    attribute_id.id,
                                    &value.value,
                                    &resource.path,
                                )?,
                            HtmlResourceKind::Asset => self.resolve_html_module_graph_asset_edge(
                                revision,
                                module,
                                profile_id,
                                dir,
                                attribute_id.id,
                                &value.value,
                                &resource.path,
                            )?,
                        };

                        dependencies.push(dependency);
                    }
                    AttributeResource::SourceSet(source_set) => {
                        for item in source_set.items.iter().filter(|item| !item.is_external) {
                            dependencies.push(self.resolve_html_module_graph_asset_edge(
                                revision,
                                module,
                                profile_id,
                                dir,
                                attribute_id.id,
                                &item.value,
                                &item.path,
                            )?);
                        }
                    }
                }
            }

            self.collect_html_module_graph_edges_in_nodes(
                revision,
                module,
                profile_id,
                dir,
                html,
                &element.children,
                dependencies,
            )?;

            if let Some(content) = element.content {
                let children = html.tree.get(content).children.clone();
                self.collect_html_module_graph_edges_in_nodes(
                    revision,
                    module,
                    profile_id,
                    dir,
                    html,
                    &children,
                    dependencies,
                )?;
            }
        }

        Ok(())
    }

    /// Collect graph edges from one CSS rule subtree.
    fn collect_css_module_graph_edges_in_rule(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        css: &Css,
        rule_id: css::LocalNodeId<css::Rule>,
        dependencies: &mut Vec<ModuleEdge>,
    ) -> ResolveResult<()> {
        let mut declaration_block = None;
        let mut nested_rules = Vec::new();
        let mut page_margin_rules = Vec::new();
        let mut supports_condition = None;

        match css.tree.get(rule_id) {
            css::Rule::Import(rule) => {
                let Some(resource) = rule.resource.as_ref() else {
                    return Ok(());
                };

                if resource.is_external {
                    return Ok(());
                }

                dependencies.push(self.resolve_css_module_graph_import_edge(
                    revision,
                    module,
                    profile_id,
                    dir,
                    resource.id,
                    &rule.url,
                    &resource.path,
                )?);
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
                if let Some(initial_value) = &rule.initial_value {
                    self.collect_css_module_graph_edges_in_components(
                        revision,
                        module,
                        profile_id,
                        dir,
                        css,
                        initial_value.components(),
                        dependencies,
                    )?;
                }
            }
            css::Rule::Unknown(rule) => {
                self.collect_css_module_graph_edges_in_components(
                    revision,
                    module,
                    profile_id,
                    dir,
                    css,
                    &rule.prelude,
                    dependencies,
                )?;

                if let Some(block) = &rule.block {
                    self.collect_css_module_graph_edges_in_components(
                        revision,
                        module,
                        profile_id,
                        dir,
                        css,
                        block,
                        dependencies,
                    )?;
                }
            }
            css::Rule::Custom(rule) => {
                self.collect_css_module_graph_edges_in_components(
                    revision,
                    module,
                    profile_id,
                    dir,
                    css,
                    &rule.components,
                    dependencies,
                )?;
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
            self.collect_css_module_graph_edges_in_supports_condition(
                revision,
                module,
                profile_id,
                dir,
                css,
                supports_condition,
                dependencies,
            )?;
        }

        if let Some(declaration_block) = declaration_block {
            self.collect_css_module_graph_edges_in_declaration_block(
                revision,
                module,
                profile_id,
                dir,
                css,
                declaration_block,
                dependencies,
            )?;
        }

        for page_margin_rule in page_margin_rules {
            self.collect_css_module_graph_edges_in_page_margin_rule(
                revision,
                module,
                profile_id,
                dir,
                css,
                page_margin_rule,
                dependencies,
            )?;
        }

        for nested_rule in nested_rules {
            self.collect_css_module_graph_edges_in_rule(
                revision,
                module,
                profile_id,
                dir,
                css,
                nested_rule,
                dependencies,
            )?;
        }

        Ok(())
    }

    /// Collect graph edges from one page margin rule subtree.
    fn collect_css_module_graph_edges_in_page_margin_rule(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        css: &Css,
        rule_id: css::LocalNodeId<css::PageMarginRule>,
        dependencies: &mut Vec<ModuleEdge>,
    ) -> ResolveResult<()> {
        let declaration_block = css.tree.get(rule_id).declarations;

        if let Some(declaration_block) = declaration_block {
            self.collect_css_module_graph_edges_in_declaration_block(
                revision,
                module,
                profile_id,
                dir,
                css,
                declaration_block,
                dependencies,
            )?;
        }

        Ok(())
    }

    /// Collect graph edges from one declaration block subtree.
    fn collect_css_module_graph_edges_in_declaration_block(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        css: &Css,
        declaration_block_id: css::LocalNodeId<css::DeclarationBlock>,
        dependencies: &mut Vec<ModuleEdge>,
    ) -> ResolveResult<()> {
        let declaration_ids = css.tree.get(declaration_block_id).declarations.clone();

        for declaration_id in declaration_ids {
            let declaration = css.tree.get(declaration_id);

            self.collect_css_module_graph_edges_in_components(
                revision,
                module,
                profile_id,
                dir,
                css,
                declaration.value.components(),
                dependencies,
            )?;
        }

        Ok(())
    }

    /// Collect graph edges from one supports condition subtree.
    fn collect_css_module_graph_edges_in_supports_condition(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        css: &Css,
        condition_id: css::LocalNodeId<css::SupportsCondition>,
        dependencies: &mut Vec<ModuleEdge>,
    ) -> ResolveResult<()> {
        match css.tree.get(condition_id) {
            css::SupportsCondition::Not(condition) => self
                .collect_css_module_graph_edges_in_supports_condition(
                    revision,
                    module,
                    profile_id,
                    dir,
                    css,
                    *condition,
                    dependencies,
                )?,
            css::SupportsCondition::And(conditions) | css::SupportsCondition::Or(conditions) => {
                for condition in conditions {
                    self.collect_css_module_graph_edges_in_supports_condition(
                        revision,
                        module,
                        profile_id,
                        dir,
                        css,
                        *condition,
                        dependencies,
                    )?;
                }
            }
            css::SupportsCondition::Declaration { value, .. } => {
                self.collect_css_module_graph_edges_in_components(
                    revision,
                    module,
                    profile_id,
                    dir,
                    css,
                    value.components(),
                    dependencies,
                )?;
            }
            css::SupportsCondition::Selector(_) | css::SupportsCondition::Unknown(_) => {}
        }

        Ok(())
    }

    /// Collect graph edges from one component value list.
    fn collect_css_module_graph_edges_in_components(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        css: &Css,
        components: &css::ComponentValueList,
        dependencies: &mut Vec<ModuleEdge>,
    ) -> ResolveResult<()> {
        for value in &components.values {
            match value {
                css::ComponentValue::Token(css::Token::UnquotedUrl {
                    value,
                    url_resource,
                }) => {
                    let Some(resource) = url_resource.as_ref() else {
                        continue;
                    };

                    if resource.is_external {
                        continue;
                    }

                    dependencies.push(self.resolve_css_module_graph_resource_edge(
                        revision,
                        module,
                        profile_id,
                        dir,
                        resource.id,
                        value,
                        &resource.path,
                    )?);
                }
                css::ComponentValue::Function(function)
                    if function.name_eq(&css.tree.strings, "url") =>
                {
                    let Some(resource) = function.url_resource.as_ref() else {
                        continue;
                    };

                    if resource.is_external {
                        continue;
                    }

                    let specifier = self.css_function_url_value(css, &function.arguments);
                    dependencies.push(self.resolve_css_module_graph_resource_edge(
                        revision,
                        module,
                        profile_id,
                        dir,
                        resource.id,
                        &specifier,
                        &resource.path,
                    )?);
                }
                css::ComponentValue::Function(function) => {
                    self.collect_css_module_graph_edges_in_components(
                        revision,
                        module,
                        profile_id,
                        dir,
                        css,
                        &function.arguments,
                        dependencies,
                    )?;
                }
                css::ComponentValue::Block(block) => {
                    self.collect_css_module_graph_edges_in_components(
                        revision,
                        module,
                        profile_id,
                        dir,
                        css,
                        &block.value,
                        dependencies,
                    )?;
                }
                css::ComponentValue::Token(_) => {}
            }
        }

        Ok(())
    }

    /// Return the canonical authored url value for one `url(...)` argument list.
    fn css_function_url_value(&self, _css: &Css, arguments: &css::ComponentValueList) -> String {
        if arguments.values.len() == 1 {
            match &arguments.values[0] {
                css::ComponentValue::Token(css::Token::String(value))
                | css::ComponentValue::Token(css::Token::UnquotedUrl { value, .. }) => {
                    return value.clone();
                }
                _ => {}
            }
        }

        arguments
            .values
            .iter()
            .find_map(|value| match value {
                css::ComponentValue::Token(css::Token::String(value))
                | css::ComponentValue::Token(css::Token::UnquotedUrl { value, .. }) => {
                    Some(value.clone())
                }
                css::ComponentValue::Token(css::Token::WhiteSpace(_)) => None,
                _ => None,
            })
            .unwrap_or_default()
    }

    /// Return the parsed HTML module payload.
    fn html_module_payload(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: &DirResolved,
    ) -> ResolveResult<Html> {
        let data = self
            .data(module_id)
            .ok_or_else(|| ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module_id, Some(profile_id)),
            })?;

        match data.as_ref() {
            Data::Html(html) => Ok(html.clone()),
            Data::Json(_) | Data::Css(_) => Err(ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module_id, Some(profile_id)),
            }),
        }
    }

    /// Return the parsed CSS module payload.
    fn css_module_payload(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: &DirResolved,
    ) -> ResolveResult<Css> {
        let data = self
            .data(module_id)
            .ok_or_else(|| ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module_id, Some(profile_id)),
            })?;

        match data.as_ref() {
            Data::Css(css) => Ok(css.clone()),
            Data::Json(_) | Data::Html(_) => Err(ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module_id, Some(profile_id)),
            }),
        }
    }

    /// Resolve one HTML module script reference into a graph edge.
    fn resolve_html_module_graph_script_edge(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        site: u32,
        specifier: &str,
        path: &str,
    ) -> ResolveResult<ModuleEdge> {
        let target_module_id =
            self.resolve_html_module_graph_path_edge(revision, module, profile_id, dir, path)?;
        let target_module = self
            .cache_module_snapshot(revision, target_module_id)
            .map_err(|_| ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module.id, Some(profile_id)),
            })?;

        if !target_module.is_code() {
            return Err(ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module.id, Some(profile_id)),
            });
        }

        Ok(
            ModuleEdge::new(target_module_id, ModuleEdgeRelation::DocumentScript)
                .with_specifier(Some(self.repository.strings.intern(specifier)))
                .with_site(Some(site)),
        )
    }

    /// Resolve one HTML stylesheet reference into a graph edge.
    fn resolve_html_module_graph_stylesheet_edge(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        site: u32,
        specifier: &str,
        path: &str,
    ) -> ResolveResult<ModuleEdge> {
        let target_module_id =
            self.resolve_html_module_graph_path_edge(revision, module, profile_id, dir, path)?;

        Ok(
            ModuleEdge::new(target_module_id, ModuleEdgeRelation::DocumentStylesheet)
                .with_specifier(Some(self.repository.strings.intern(specifier)))
                .with_site(Some(site)),
        )
    }

    /// Resolve one HTML asset reference into a graph edge.
    fn resolve_html_module_graph_asset_edge(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        site: u32,
        specifier: &str,
        path: &str,
    ) -> ResolveResult<ModuleEdge> {
        let target_module_id =
            self.resolve_html_module_graph_path_edge(revision, module, profile_id, dir, path)?;

        Ok(
            ModuleEdge::new(target_module_id, ModuleEdgeRelation::Resource)
                .with_specifier(Some(self.repository.strings.intern(specifier)))
                .with_site(Some(site)),
        )
    }

    /// Resolve one CSS stylesheet import reference into a graph edge.
    fn resolve_css_module_graph_import_edge(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        site: u32,
        specifier: &str,
        path: &str,
    ) -> ResolveResult<ModuleEdge> {
        let target_module_id =
            self.resolve_html_module_graph_path_edge(revision, module, profile_id, dir, path)?;
        let target_module = self
            .cache_module_snapshot(revision, target_module_id)
            .map_err(|_| ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module.id, Some(profile_id)),
            })?;
        let target_file = self
            .cache_file_snapshot(revision, target_module.file_id)
            .map_err(|_| ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module.id, Some(profile_id)),
            })?;
        let target_kind =
            ModuleKind::from_file_type_and_loader(target_file.ty, target_module.loader);

        if target_kind != ModuleKind::Css {
            return Err(ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module.id, Some(profile_id)),
            });
        }

        Ok(
            ModuleEdge::new(target_module_id, ModuleEdgeRelation::StyleImport)
                .with_specifier(Some(self.repository.strings.intern(specifier)))
                .with_site(Some(site)),
        )
    }

    /// Resolve one CSS resource reference into a graph edge.
    fn resolve_css_module_graph_resource_edge(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        site: u32,
        specifier: &str,
        path: &str,
    ) -> ResolveResult<ModuleEdge> {
        let target_module_id =
            self.resolve_html_module_graph_path_edge(revision, module, profile_id, dir, path)?;

        Ok(
            ModuleEdge::new(target_module_id, ModuleEdgeRelation::StyleUrl)
                .with_specifier(Some(self.repository.strings.intern(specifier)))
                .with_site(Some(site)),
        )
    }

    /// Resolve one local HTML path reference into a target module id.
    fn resolve_html_module_graph_path_edge(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        specifier_path: &str,
    ) -> ResolveResult<ModuleId> {
        let Some(document_path) = module.path.as_ref() else {
            return Err(ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module.id, Some(profile_id)),
            });
        };
        let package = self
            .cache_package_snapshot(revision, module.package_id)
            .map_err(|_| ResolveError::UnsupportedConstruct {
                node: dir.anchor_node.into_anchored(module.id, Some(profile_id)),
            })?;
        let package_dir = package.path.clone().unwrap_or_else(|| {
            document_path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_path_buf()
        });
        let root_dir = self
            .repository
            .package_options(revision, module.package_id)
            .ok()
            .flatten()
            .and_then(|options| options.compiler.root_dir.clone());
        let source_path = resolve_source_reference_path(
            document_path,
            specifier_path,
            &package_dir,
            root_dir.as_deref(),
        );

        let metadata = self
            .repository
            .file_system()
            .metadata(&source_path)
            .map_err(|_| {
                let target = self.repository.strings.intern(specifier_path);
                ResolveError::UnresolvedModule {
                    node: dir.anchor_node.into_anchored(module.id, Some(profile_id)),
                    target,
                }
            })?;

        if !metadata.is_file {
            let target = self.repository.strings.intern(specifier_path);
            return Err(ResolveError::UnresolvedModule {
                node: dir.anchor_node.into_anchored(module.id, Some(profile_id)),
                target,
            });
        }

        let source_path = source_path.to_path_buf();
        let target_module_id = self
            .resolve_path_to_module(revision, &source_path)
            .map_err(|_| ResolveError::UnresolvedModule {
                node: dir.anchor_node.into_anchored(module.id, Some(profile_id)),
                target: self.repository.strings.intern(specifier_path),
            })?;

        Ok(target_module_id)
    }

    /// Collect graph edges from one module target.
    fn collect_module_graph_target_edges(
        &self,
        dependencies: &mut Vec<ModuleEdge>,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        relation: ModuleEdgeRelation,
        specifier: Option<StringId>,
        loader: Option<Loader>,
        target: ModuleTarget,
    ) -> ResolveResult<()> {
        match target {
            ModuleTarget::Module(target_id) => {
                dependencies.push(
                    ModuleEdge::new(target_id, relation)
                        .with_specifier(specifier)
                        .with_loader(loader),
                );
            }
            ModuleTarget::Binding(binding_specifier) => {
                let bindings = self.module_bindings_for_specifier(
                    revision,
                    module_id,
                    profile_id,
                    binding_specifier,
                )?;
                let Some(bindings) = bindings else {
                    return Ok(());
                };

                for binding in bindings {
                    dependencies.push(
                        ModuleEdge::new(binding.module_id, relation)
                            .with_specifier(specifier)
                            .with_loader(loader),
                    );
                }
            }
            ModuleTarget::External(_) => {}
        }

        Ok(())
    }

    /// Return the authored specifier for one namespace export.
    fn namespace_export_module_edge_specifier(
        &self,
        dir: &DirResolved,
        export: destack_dir::NamespaceExport,
    ) -> Option<StringId> {
        let item = dir.tree.get(export.item);
        match item {
            DependencyItem::Remote { target, .. }
            | DependencyItem::UnresolvedRemote { target, .. } => Some(*target),
            _ => None,
        }
    }
}

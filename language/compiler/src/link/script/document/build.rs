use std::collections::BTreeSet;

use destack_artifact::{ArtifactKey, Data, Html, ModuleEdgeRelation, ModuleGraph};
use destack_html as html;
use destack_source::{FileType, ModuleId};
use destack_workspace::Module;
use indexmap::IndexSet;

use crate::{LinkError, LinkResult, RequirementCollector};

use super::super::ScriptLinker;

impl<'a> ScriptLinker<'a> {
    /// Collect the rooted modules from the target roots.
    pub(in crate::link::script) fn collect_root_modules(
        &self,
        root_modules: &[ModuleId],
    ) -> LinkResult<(Vec<ModuleId>, Vec<ModuleId>, Vec<ModuleId>, Vec<ModuleId>)> {
        // html entries root additional modules through document references
        if self.target.emit == destack_artifact::EmitFormat::Html {
            return self.collect_html_root_modules(root_modules);
        }

        Ok((Vec::new(), root_modules.to_vec(), Vec::new(), Vec::new()))
    }

    /// Collect the rooted modules from html target entries.
    fn collect_html_root_modules(
        &self,
        root_modules: &[ModuleId],
    ) -> LinkResult<(Vec<ModuleId>, Vec<ModuleId>, Vec<ModuleId>, Vec<ModuleId>)> {
        let mut document_module_ids = Vec::with_capacity(root_modules.len());
        let mut script_module_ids = BTreeSet::new();
        let mut stylesheet_module_ids = BTreeSet::new();
        let mut asset_module_ids = BTreeSet::new();

        // html entry discovery needs parsed documents and the linked graph
        self.require_html_root_artifacts(root_modules)?;

        for module_id in root_modules {
            let module = self.module(*module_id);
            let file = self.file(module.file_id);

            // html targets must root html source files
            if file.ty != FileType::Html {
                return Err(LinkError::InvalidModuleKind {
                    anchor: (*module_id).into(),
                    package: self.package_id,
                    target: self.target_id.clone(),
                    subject: "html entry module".to_string(),
                    expected: ".html".to_string(),
                    found: module.uri.to_string(),
                });
            }

            document_module_ids.push(module.id);
            self.collect_html_module_roots(
                module.as_ref(),
                &mut script_module_ids,
                &mut stylesheet_module_ids,
                &mut asset_module_ids,
            )?;
        }

        // outFile html targets can only emit one document
        if self.target.out_file.is_some() && document_module_ids.len() > 1 {
            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                message: "html targets with outFile can only emit one document entry".to_string(),
            });
        }

        Ok((
            document_module_ids,
            script_module_ids.into_iter().collect(),
            stylesheet_module_ids.into_iter().collect(),
            asset_module_ids.into_iter().collect(),
        ))
    }

    /// Require the artifacts needed to collect html rooted modules.
    fn require_html_root_artifacts(&self, root_modules: &[ModuleId]) -> LinkResult<()> {
        let mut collector = RequirementCollector::new();

        // html root discovery reads parsed payloads directly
        for module_id in root_modules {
            let result = self.require_ast(*module_id);
            collector.try_collect(result);
        }

        // html edge resolution needs each entry published into its resolved graph
        for module_id in root_modules {
            let profile_id = self.profile_id_for_module(*module_id)?;
            let result =
                self.compiler
                    .require_dir_resolved(self.revision(), *module_id, profile_id);
            let error = collector.try_collect(result);

            if let Some(error) = error {
                return Err(error.into());
            }
        }

        // html root discovery needs each entry graph published for its own profile
        for module_id in root_modules {
            let profile_id = self.profile_id_for_module(*module_id)?;
            let result = self
                .compiler
                .require_artifact(self.revision(), ArtifactKey::module_graph(profile_id));
            let error = collector.try_collect(result);

            if let Some(error) = error {
                return Err(error.into());
            }
        }

        if let Some(requirement) = collector.try_into_requirement() {
            return Err(LinkError::Yield { requirement });
        }

        Ok(())
    }

    /// Collect rooted modules from one parsed html module.
    fn collect_html_module_roots(
        &self,
        module: &Module,
        script_module_ids: &mut BTreeSet<ModuleId>,
        stylesheet_module_ids: &mut BTreeSet<ModuleId>,
        asset_module_ids: &mut BTreeSet<ModuleId>,
    ) -> LinkResult<()> {
        let html = self.html_payload(module.id)?;
        let module_graph = self.module_graph_for_module(module.id)?;
        let root_node = html.tree.get(html.document);

        self.collect_html_nodes(
            module,
            &html,
            module_graph.as_ref(),
            &root_node.children,
            script_module_ids,
            stylesheet_module_ids,
            asset_module_ids,
        )?;

        Ok(())
    }

    /// Collect rooted modules from one html node list.
    fn collect_html_nodes(
        &self,
        module: &Module,
        html: &Html,
        module_graph: &ModuleGraph,
        node_ids: &[html::LocalNodeId<html::Content>],
        script_module_ids: &mut BTreeSet<ModuleId>,
        stylesheet_module_ids: &mut BTreeSet<ModuleId>,
        asset_module_ids: &mut BTreeSet<ModuleId>,
    ) -> LinkResult<()> {
        for node_id in node_ids {
            let html::Content::Element(element) = html.tree.get(*node_id) else {
                continue;
            };

            self.collect_html_attributes(
                module,
                html,
                module_graph,
                &element.name,
                &element.attributes,
                script_module_ids,
                stylesheet_module_ids,
                asset_module_ids,
            )?;

            self.collect_html_nodes(
                module,
                html,
                module_graph,
                &element.children,
                script_module_ids,
                stylesheet_module_ids,
                asset_module_ids,
            )?;

            // template content
            if let Some(fragment_id) = element.content {
                let fragment = html.tree.get(fragment_id);

                self.collect_html_nodes(
                    module,
                    html,
                    module_graph,
                    &fragment.children,
                    script_module_ids,
                    stylesheet_module_ids,
                    asset_module_ids,
                )?;
            }
        }

        Ok(())
    }

    /// Collect rooted modules from one html attribute list.
    fn collect_html_attributes(
        &self,
        module: &Module,
        html: &Html,
        module_graph: &ModuleGraph,
        element_name: &html::Name,
        attribute_ids: &[html::LocalNodeId<html::Attribute>],
        script_module_ids: &mut BTreeSet<ModuleId>,
        stylesheet_module_ids: &mut BTreeSet<ModuleId>,
        asset_module_ids: &mut BTreeSet<ModuleId>,
    ) -> LinkResult<()> {
        for attribute_id in attribute_ids {
            let attribute = html.tree.get(*attribute_id);
            let Some(value) = attribute.value.as_ref() else {
                continue;
            };
            let Some(resource) = value.resource.as_ref() else {
                continue;
            };

            // owned resource sites
            match resource {
                html::AttributeResource::Resource(resource)
                    if resource.kind == html::HtmlResourceKind::ModuleScript =>
                {
                    let Some(module_id) = self.resolve_html_module_script(
                        module,
                        module_graph,
                        attribute_id.id,
                        &value.value,
                    )?
                    else {
                        continue;
                    };

                    if self.html_module_script_roots_output(html, element_name) {
                        script_module_ids.insert(module_id);
                    }
                }

                // stylesheet references
                html::AttributeResource::Resource(resource)
                    if resource.kind == html::HtmlResourceKind::Stylesheet =>
                {
                    let Some(module_id) = self.resolve_html_stylesheet(
                        module,
                        module_graph,
                        attribute_id.id,
                        &value.value,
                    )?
                    else {
                        continue;
                    };

                    if !resource.suffix.is_empty() {
                        return Err(LinkError::UnsupportedReferenceSuffix {
                            anchor: module.id.into(),
                            package: self.package_id,
                            target: self.target_id.clone(),
                            reference: "html stylesheet reference".to_string(),
                            value: value.value.clone(),
                        });
                    }

                    stylesheet_module_ids.insert(module_id);
                }

                // asset references
                html::AttributeResource::Resource(resource)
                    if resource.kind == html::HtmlResourceKind::Asset =>
                {
                    let Some((module_id, _)) = self.resolve_html_asset(
                        module,
                        module_graph,
                        attribute_id.id,
                        &value.value,
                        &resource.suffix,
                    )?
                    else {
                        continue;
                    };

                    asset_module_ids.insert(module_id);
                }

                // srcset references
                html::AttributeResource::SourceSet(source_set) => {
                    for item in &source_set.items {
                        if item.is_external {
                            continue;
                        }

                        let Some((module_id, _)) = self.resolve_html_asset(
                            module,
                            module_graph,
                            attribute_id.id,
                            &item.value,
                            &item.suffix,
                        )?
                        else {
                            continue;
                        };

                        asset_module_ids.insert(module_id);
                    }
                }

                html::AttributeResource::Resource(_) => {}
            }
        }

        Ok(())
    }

    /// Return the parsed HTML payload for one source module.
    pub(in crate::link::script) fn html_payload(&self, module_id: ModuleId) -> LinkResult<Html> {
        let data = self.data(module_id)?;
        let module = self.module(module_id);

        match data.as_ref() {
            Data::Html(html) => Ok(html.clone()),
            Data::Json(_) | Data::Css(_) => Err(LinkError::InvalidModuleKind {
                anchor: module_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                subject: "html entry module".to_string(),
                expected: "html module".to_string(),
                found: module.uri.to_string(),
            }),
        }
    }

    /// Resolve one local module script reference against one HTML attribute site.
    pub(in crate::link::script) fn resolve_html_module_script(
        &self,
        module: &Module,
        module_graph: &ModuleGraph,
        attribute_id: u32,
        specifier: &str,
    ) -> LinkResult<Option<ModuleId>> {
        let specifier_id = self.string_pool().intern(specifier);
        let Some(module_edge) = module_graph.dependency_edge_for_site_specifier(
            module.id,
            ModuleEdgeRelation::DocumentScript,
            attribute_id,
            specifier_id,
        ) else {
            return Ok(None);
        };
        let module_id = module_edge.target;
        let resolved_module = self.module(module_id);

        if !resolved_module.is_code() {
            return Err(LinkError::InvalidModuleKind {
                anchor: module.id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                subject: "html module script reference".to_string(),
                expected: "code module".to_string(),
                found: resolved_module.uri.to_string(),
            });
        }

        Ok(Some(module_id))
    }

    /// Resolve one local asset reference against one HTML attribute site.
    pub(in crate::link::script) fn resolve_html_asset(
        &self,
        module: &Module,
        module_graph: &ModuleGraph,
        attribute_id: u32,
        specifier: &str,
        suffix: &str,
    ) -> LinkResult<Option<(ModuleId, String)>> {
        let specifier_id = self.string_pool().intern(specifier);
        let Some(module_edge) = module_graph.dependency_edge_for_site_specifier(
            module.id,
            ModuleEdgeRelation::Resource,
            attribute_id,
            specifier_id,
        ) else {
            return Ok(None);
        };
        let module_id = module_edge.target;

        Ok(Some((module_id, suffix.to_string())))
    }

    /// Resolve one local stylesheet reference against one HTML attribute site.
    pub(in crate::link::script) fn resolve_html_stylesheet(
        &self,
        module: &Module,
        module_graph: &ModuleGraph,
        attribute_id: u32,
        specifier: &str,
    ) -> LinkResult<Option<ModuleId>> {
        let specifier_id = self.string_pool().intern(specifier);
        let Some(module_edge) = module_graph.dependency_edge_for_site_specifier(
            module.id,
            ModuleEdgeRelation::DocumentStylesheet,
            attribute_id,
            specifier_id,
        ) else {
            return Ok(None);
        };
        let module_id = module_edge.target;
        let stylesheet = self.module(module_id);

        let stylesheet_file = self.file(stylesheet.file_id);

        if stylesheet_file.ty != FileType::Css {
            return Err(LinkError::InvalidModuleKind {
                anchor: module.id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                subject: "html stylesheet reference".to_string(),
                expected: "css module".to_string(),
                found: stylesheet.uri.to_string(),
            });
        }

        Ok(Some(module_id))
    }

    /// Build one HTML document payload plus its explicit rooted references.
    pub(super) fn build_html_document(
        &self,
        module: &Module,
    ) -> LinkResult<(Html, Vec<ModuleId>, Vec<ModuleId>)> {
        // source state
        let html = self.html_payload(module.id)?;
        let module_graph = self.module_graph_for_module(module.id)?;
        let mut script_module_ids = Vec::new();
        let mut stylesheet_module_ids = Vec::new();
        let document_node = html.tree.get(html.document);

        // rooted references
        self.build_html_nodes(
            module,
            &html,
            module_graph.as_ref(),
            &document_node.children,
            &mut script_module_ids,
            &mut stylesheet_module_ids,
        )?;

        // keep the first authored reference order stable
        let mut seen_script_module_ids = IndexSet::new();
        script_module_ids.retain(|module_id| seen_script_module_ids.insert(*module_id));

        // keep the first authored reference order stable
        let mut seen_stylesheet_module_ids = IndexSet::new();
        stylesheet_module_ids.retain(|module_id| seen_stylesheet_module_ids.insert(*module_id));

        Ok((html, script_module_ids, stylesheet_module_ids))
    }

    /// Collect rooted references from one parsed HTML node list.
    fn build_html_nodes(
        &self,
        module: &Module,
        document: &Html,
        module_graph: &ModuleGraph,
        nodes: &[html::LocalNodeId<html::Content>],
        script_module_ids: &mut Vec<ModuleId>,
        stylesheet_module_ids: &mut Vec<ModuleId>,
    ) -> LinkResult<()> {
        for node_id in nodes {
            let node = document.tree.get(*node_id);

            // element contents
            if let html::Content::Element(element) = node {
                self.build_html_attributes(
                    module,
                    document,
                    module_graph,
                    &element.name,
                    &element.attributes,
                    script_module_ids,
                    stylesheet_module_ids,
                )?;

                self.build_html_nodes(
                    module,
                    document,
                    module_graph,
                    &element.children,
                    script_module_ids,
                    stylesheet_module_ids,
                )?;

                if let Some(fragment_id) = element.content {
                    self.build_html_fragment(
                        module,
                        document,
                        module_graph,
                        fragment_id,
                        script_module_ids,
                        stylesheet_module_ids,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Collect rooted references from one parsed HTML fragment.
    fn build_html_fragment(
        &self,
        module: &Module,
        document: &Html,
        module_graph: &ModuleGraph,
        fragment_id: html::LocalNodeId<html::Fragment>,
        script_module_ids: &mut Vec<ModuleId>,
        stylesheet_module_ids: &mut Vec<ModuleId>,
    ) -> LinkResult<()> {
        let fragment = document.tree.get(fragment_id);

        self.build_html_nodes(
            module,
            document,
            module_graph,
            &fragment.children,
            script_module_ids,
            stylesheet_module_ids,
        )
    }

    /// Collect rooted references from one parsed HTML attribute list.
    fn build_html_attributes(
        &self,
        module: &Module,
        document: &Html,
        module_graph: &ModuleGraph,
        element_name: &html::Name,
        attributes: &[html::LocalNodeId<html::Attribute>],
        script_module_ids: &mut Vec<ModuleId>,
        stylesheet_module_ids: &mut Vec<ModuleId>,
    ) -> LinkResult<()> {
        for attribute_id in attributes {
            self.build_html_attribute_value(
                module,
                document,
                module_graph,
                element_name,
                *attribute_id,
                script_module_ids,
                stylesheet_module_ids,
            )?;
        }

        Ok(())
    }

    /// Collect rooted references from one parsed HTML attribute value.
    fn build_html_attribute_value(
        &self,
        module: &Module,
        document: &Html,
        module_graph: &ModuleGraph,
        element_name: &html::Name,
        attribute_id: html::LocalNodeId<html::Attribute>,
        script_module_ids: &mut Vec<ModuleId>,
        stylesheet_module_ids: &mut Vec<ModuleId>,
    ) -> LinkResult<()> {
        // parsed value
        let attribute = document.tree.get(attribute_id);
        let Some(value) = attribute.value.as_ref() else {
            return Ok(());
        };
        let Some(resource) = value.resource.as_ref() else {
            return Ok(());
        };

        // rooted resource sites
        match resource {
            html::AttributeResource::Resource(resource)
                if resource.kind == html::HtmlResourceKind::ModuleScript =>
            {
                if let Some(module_id) = self.resolve_html_module_script(
                    module,
                    module_graph,
                    attribute_id.id,
                    &value.value,
                )? {
                    if self.html_module_script_roots_output(document, element_name) {
                        script_module_ids.push(module_id);
                    }
                }
            }

            html::AttributeResource::Resource(resource)
                if resource.kind == html::HtmlResourceKind::Stylesheet =>
            {
                if let Some(module_id) = self.resolve_html_stylesheet(
                    module,
                    module_graph,
                    attribute_id.id,
                    &value.value,
                )? {
                    if !resource.suffix.is_empty() {
                        return Err(LinkError::UnsupportedReferenceSuffix {
                            anchor: module.id.into(),
                            package: self.package_id,
                            target: self.target_id.clone(),
                            reference: "html stylesheet reference".to_string(),
                            value: value.value.clone(),
                        });
                    }

                    stylesheet_module_ids.push(module_id);
                }
            }

            html::AttributeResource::Resource(resource)
                if resource.kind == html::HtmlResourceKind::Asset =>
            {
                let _ = self.resolve_html_asset(
                    module,
                    module_graph,
                    attribute_id.id,
                    &value.value,
                    &resource.suffix,
                )?;
            }

            html::AttributeResource::SourceSet(source_set) => {
                self.build_html_source_set(
                    module,
                    module_graph,
                    attribute_id.id,
                    &source_set.items,
                )?;
            }

            html::AttributeResource::Resource(_) => {}
        }

        Ok(())
    }

    /// Return whether one HTML element roots script output.
    pub(in crate::link::script) fn html_module_script_roots_output(
        &self,
        document: &Html,
        element_name: &html::Name,
    ) -> bool {
        // only plain html script elements root module output
        if element_name.namespace != html::Namespace::Html {
            return false;
        }

        document.tree.strings.get(element_name.local).as_ref() == "script"
    }

    /// Validate one linked HTML srcset value from exact graph edges.
    fn build_html_source_set(
        &self,
        module: &Module,
        module_graph: &ModuleGraph,
        attribute_id: u32,
        items: &[html::SourceSetItem],
    ) -> LinkResult<()> {
        for item in items {
            // external items stay authored
            if item.is_external {
                continue;
            }

            if self
                .resolve_html_asset(
                    module,
                    module_graph,
                    attribute_id,
                    &item.value,
                    &item.suffix,
                )?
                .is_none()
            {
                return Err(LinkError::MissingReferenceEdge {
                    anchor: module.id.into(),
                    package: self.package_id,
                    target: self.target_id.clone(),
                    reference: "html srcset".to_string(),
                    module: module.uri.to_string(),
                    site: attribute_id,
                    specifier: item.value.clone(),
                });
            }
        }

        Ok(())
    }
}

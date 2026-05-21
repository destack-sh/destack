use destack_artifact::Html;
use destack_html as html;
use destack_source::{FileId, ModuleId, Span};
use destack_workspace::Module;

use crate::{LinkError, LinkResult};

use super::super::plan::Plan;
use super::super::{AssetReference, ModuleEdge, ScriptLinker};
use crate::link::{OutputLocation, TargetLocation};

/// Print one rewritten HTML document through the shared HTML printer.
pub(super) fn print_html_document(
    linker: &ScriptLinker<'_>,
    module: &Module,
    module_edges: &[ModuleEdge],
    document: &Html,
    document_location: &OutputLocation,
    plan: &Plan,
    associated_stylesheets: &[String],
) -> LinkResult<String> {
    let mut document = document.clone();

    // rewrite authored references
    rewrite_html_document(
        linker,
        module,
        module_edges,
        &mut document,
        document_location,
        plan,
    )?;

    // inject associated stylesheets
    inject_document_stylesheets(&mut document, associated_stylesheets);

    Ok(html::print::print_document_with_options(
        &document.tree,
        document.document,
        html::print::PrintOptions {
            is_minified: linker.target.should_minify_bundle_html_output(),
        },
    ))
}

/// Render one HTML attribute value from the original HTML tree.
pub(super) fn render_html_attribute_value(
    linker: &ScriptLinker<'_>,
    module: &Module,
    module_edges: &[ModuleEdge],
    document_location: &OutputLocation,
    plan: &Plan,
    attribute_id: html::LocalNodeId<html::Attribute>,
    value: &html::AttributeValue,
) -> LinkResult<String> {
    let Some(resource) = value.resource.as_ref() else {
        return Ok(value.value.clone());
    };

    match resource {
        html::AttributeResource::Resource(resource) => {
            if resource.is_external {
                return Ok(value.value.clone());
            }

            match resource.kind {
                html::HtmlResourceKind::ModuleScript => {
                    let Some(module_id) = linker.resolve_html_module_script(
                        module,
                        module_edges,
                        attribute_id.id,
                        &value.value,
                    )?
                    else {
                        return Ok(value.value.clone());
                    };
                    let target_location = TargetLocation::new(
                        linker.package_dir,
                        linker.target,
                        linker.target_name(),
                    );
                    let output_id = plan
                        .output_graph()
                        .output_id_for_module(module_id)
                        .ok_or_else(|| LinkError::Internal {
                            anchor: (linker.package_id).into(),
                            package: linker.package_id,
                            message: format!(
                                "missing linked output for html module script {:?}",
                                module_id
                            ),
                        })?;
                    let output_location = plan
                        .output_layout()
                        .output_location(output_id)
                        .ok_or_else(|| LinkError::Internal {
                            anchor: (linker.package_id).into(),
                            package: linker.package_id,
                            message: format!(
                                "missing output layout for html module script {:?}",
                                module_id
                            ),
                        })?;

                    Ok(target_location.document_reference(document_location, output_location))
                }

                html::HtmlResourceKind::Stylesheet => {
                    let Some(module_id) = linker.resolve_html_stylesheet(
                        module,
                        module_edges,
                        attribute_id.id,
                        &value.value,
                    )?
                    else {
                        return Ok(value.value.clone());
                    };
                    let target_location = TargetLocation::new(
                        linker.package_dir,
                        linker.target,
                        linker.target_name(),
                    );
                    let output_location =
                        plan.stylesheet_output_location(module_id).ok_or_else(|| {
                            LinkError::Internal {
                                anchor: (linker.package_id).into(),
                                package: linker.package_id,
                                message: format!(
                                    "missing planned output for html stylesheet {:?}",
                                    module_id
                                ),
                            }
                        })?;

                    Ok(target_location.document_reference(document_location, output_location))
                }

                html::HtmlResourceKind::Asset => {
                    let Some((module_id, suffix)) = linker.resolve_html_asset(
                        module,
                        module_edges,
                        attribute_id.id,
                        &value.value,
                        &resource.suffix,
                    )?
                    else {
                        return Ok(value.value.clone());
                    };

                    render_html_asset_reference(linker, document_location, plan, module_id, &suffix)
                }
            }
        }

        html::AttributeResource::SourceSet(source_set) => render_html_source_set(
            linker,
            module,
            module_edges,
            document_location,
            plan,
            attribute_id.id,
            &source_set.items,
        ),
    }
}

/// Render one linked srcset value from the original HTML tree.
fn render_html_source_set(
    linker: &ScriptLinker<'_>,
    module: &Module,
    module_edges: &[ModuleEdge],
    document_location: &OutputLocation,
    plan: &Plan,
    attribute_id: u32,
    items: &[html::SourceSetItem],
) -> LinkResult<String> {
    let mut rendered_items = Vec::with_capacity(items.len());

    for item in items {
        // external candidates stay authored
        let url = if item.is_external {
            item.value.clone()
        } else {
            let Some((module_id, suffix)) = linker.resolve_html_asset(
                module,
                module_edges,
                attribute_id,
                &item.value,
                &item.suffix,
            )?
            else {
                return Err(LinkError::MissingReferenceEdge {
                    anchor: module.id.into(),
                    package: linker.package_id,
                    target: linker.target_id.clone(),
                    reference: "html srcset".to_string(),
                    module: module.uri.to_string(),
                    reference_site: attribute_id,
                    specifier: item.value.clone(),
                });
            };

            render_html_asset_reference(linker, document_location, plan, module_id, &suffix)?
        };

        rendered_items.push(format!("{url}{}", item.descriptor));
    }

    Ok(rendered_items.join(", "))
}

/// Rewrite one HTML document in place against the final output plan.
fn rewrite_html_document(
    linker: &ScriptLinker<'_>,
    module: &Module,
    module_edges: &[ModuleEdge],
    document: &mut Html,
    document_location: &OutputLocation,
    plan: &Plan,
) -> LinkResult<()> {
    let document_node = document.tree.get(document.document).clone();

    rewrite_html_nodes(
        linker,
        module,
        module_edges,
        document,
        document_location,
        plan,
        &document_node.children,
    )
}

/// Rewrite one HTML node list in place against the final output plan.
fn rewrite_html_nodes(
    linker: &ScriptLinker<'_>,
    module: &Module,
    module_edges: &[ModuleEdge],
    document: &mut Html,
    document_location: &OutputLocation,
    plan: &Plan,
    nodes: &[html::LocalNodeId<html::Content>],
) -> LinkResult<()> {
    for node_id in nodes {
        let node = document.tree.get(*node_id).clone();

        if let html::Content::Element(element) = node {
            rewrite_html_attributes(
                linker,
                module,
                module_edges,
                document,
                document_location,
                plan,
                &element.attributes,
            )?;
            rewrite_html_nodes(
                linker,
                module,
                module_edges,
                document,
                document_location,
                plan,
                &element.children,
            )?;

            if let Some(fragment_id) = element.content {
                let fragment = document.tree.get(fragment_id).clone();

                rewrite_html_nodes(
                    linker,
                    module,
                    module_edges,
                    document,
                    document_location,
                    plan,
                    &fragment.children,
                )?;
            }
        }
    }

    Ok(())
}

/// Rewrite one HTML attribute list in place against the final output plan.
fn rewrite_html_attributes(
    linker: &ScriptLinker<'_>,
    module: &Module,
    module_edges: &[ModuleEdge],
    document: &mut Html,
    document_location: &OutputLocation,
    plan: &Plan,
    attributes: &[html::LocalNodeId<html::Attribute>],
) -> LinkResult<()> {
    for attribute_id in attributes {
        let value = document.tree.get(*attribute_id).value.clone();
        let Some(value) = value else {
            continue;
        };
        let rendered = render_html_attribute_value(
            linker,
            module,
            module_edges,
            document_location,
            plan,
            *attribute_id,
            &value,
        )?;

        let attribute = document.tree.get_mut(*attribute_id);
        let Some(attribute_value) = attribute.value.as_mut() else {
            continue;
        };

        attribute_value.value = rendered;
        attribute_value.resource = None;
    }

    Ok(())
}

/// Inject one document's associated stylesheet links into the final HTML tree.
fn inject_document_stylesheets(document: &mut Html, associated_stylesheets: &[String]) {
    if associated_stylesheets.is_empty() {
        return;
    }

    // prefer an authored head
    if let Some(head_id) = find_html_element(document, "head") {
        let stylesheet_nodes = build_associated_stylesheet_nodes(document, associated_stylesheets);
        let html::Content::Element(head) = document.tree.get_mut(head_id) else {
            return;
        };

        head.children.extend(stylesheet_nodes);

        return;
    }

    // otherwise synthesize one head under the html root
    if let Some(html_id) = find_html_element(document, "html") {
        let head_id = build_associated_stylesheet_head(document, associated_stylesheets);
        let html::Content::Element(root) = document.tree.get_mut(html_id) else {
            return;
        };

        root.children.insert(0, head_id);

        return;
    }

    // fall back to appending top-level nodes
    let stylesheet_nodes = build_associated_stylesheet_nodes(document, associated_stylesheets);
    let document_node = document.tree.get_mut(document.document);

    document_node.children.extend(stylesheet_nodes);
}

/// Return the first HTML element with one matching local name.
fn find_html_element(
    document: &Html,
    local_name: &str,
) -> Option<html::LocalNodeId<html::Content>> {
    let document_node = document.tree.get(document.document);

    find_html_element_in_nodes(document, &document_node.children, local_name)
}

/// Return the first HTML element with one matching local name in one node list.
fn find_html_element_in_nodes(
    document: &Html,
    nodes: &[html::LocalNodeId<html::Content>],
    local_name: &str,
) -> Option<html::LocalNodeId<html::Content>> {
    for node_id in nodes {
        let html::Content::Element(element) = document.tree.get(*node_id) else {
            continue;
        };

        if is_html_element_name(&document.tree, &element.name, local_name) {
            return Some(*node_id);
        }

        if let Some(found) = find_html_element_in_nodes(document, &element.children, local_name) {
            return Some(found);
        }

        if let Some(fragment_id) = element.content {
            let fragment = document.tree.get(fragment_id);

            if let Some(found) =
                find_html_element_in_nodes(document, &fragment.children, local_name)
            {
                return Some(found);
            }
        }
    }

    None
}

/// Build one synthetic head element for injected stylesheet links.
fn build_associated_stylesheet_head(
    document: &mut Html,
    associated_stylesheets: &[String],
) -> html::LocalNodeId<html::Content> {
    let children = build_associated_stylesheet_nodes(document, associated_stylesheets);
    let local_name = document.tree.intern("head");
    let element = html::Element {
        name: html::Name {
            prefix: None,
            namespace: html::Namespace::Html,
            local: local_name,
        },
        authored_start_tag_name: Some(local_name),
        has_authored_end_tag: true,
        authored_end_tag_name: Some(local_name),
        is_self_closing: false,
        self_closing_style: None,
        attributes: Vec::new(),
        children,
        content: None,
    };
    let file_id = html_document_file_id(document);

    document
        .tree
        .insert(html::Content::Element(element), Span::empty(file_id))
}

/// Build the injected stylesheet links for one document.
fn build_associated_stylesheet_nodes(
    document: &mut Html,
    associated_stylesheets: &[String],
) -> Vec<html::LocalNodeId<html::Content>> {
    let mut stylesheet_nodes = Vec::with_capacity(associated_stylesheets.len());

    for href in associated_stylesheets {
        stylesheet_nodes.push(build_associated_stylesheet_link(document, href));
    }

    stylesheet_nodes
}

/// Build one synthetic stylesheet link element.
fn build_associated_stylesheet_link(
    document: &mut Html,
    href: &str,
) -> html::LocalNodeId<html::Content> {
    let link_name = document.tree.intern("link");
    let rel_name = document.tree.intern("rel");
    let href_name = document.tree.intern("href");
    let file_id = html_document_file_id(document);
    let rel_attribute = document.tree.insert(
        html::Attribute {
            name: html::Name {
                prefix: None,
                namespace: html::Namespace::Html,
                local: rel_name,
            },
            authored_name: Some(rel_name),
            value: Some(html::AttributeValue {
                value: "stylesheet".to_string(),
                form: html::AttributeValueForm::DoubleQuoted,
                resource: None,
            }),
        },
        Span::empty(file_id),
    );
    let href_attribute = document.tree.insert(
        html::Attribute {
            name: html::Name {
                prefix: None,
                namespace: html::Namespace::Html,
                local: href_name,
            },
            authored_name: Some(href_name),
            value: Some(html::AttributeValue {
                value: href.to_string(),
                form: html::AttributeValueForm::DoubleQuoted,
                resource: None,
            }),
        },
        Span::empty(file_id),
    );
    let element = html::Element {
        name: html::Name {
            prefix: None,
            namespace: html::Namespace::Html,
            local: link_name,
        },
        authored_start_tag_name: Some(link_name),
        has_authored_end_tag: false,
        authored_end_tag_name: None,
        is_self_closing: false,
        self_closing_style: None,
        attributes: vec![rel_attribute, href_attribute],
        children: Vec::new(),
        content: None,
    };

    document
        .tree
        .insert(html::Content::Element(element), Span::empty(file_id))
}

/// Render one linked asset reference from the final asset plan.
fn render_html_asset_reference(
    linker: &ScriptLinker<'_>,
    document_location: &OutputLocation,
    plan: &Plan,
    module_id: ModuleId,
    suffix: &str,
) -> LinkResult<String> {
    let target_location =
        TargetLocation::new(linker.package_dir, linker.target, linker.target_name());
    let asset_display_name = linker.asset_display_name(module_id)?;
    let asset_reference =
        plan.asset_reference_map()
            .get(&module_id)
            .ok_or_else(|| LinkError::Internal {
                anchor: (linker.package_id).into(),
                package: linker.package_id,
                message: format!(
                    "missing planned output for html asset '{}'",
                    asset_display_name
                ),
            })?;

    match asset_reference {
        AssetReference::Inline { url } => Ok(url.clone()),

        AssetReference::Emitted { output_location } => {
            let mut reference =
                target_location.document_reference(document_location, output_location);

            reference.push_str(suffix);

            Ok(reference)
        }

        AssetReference::Original => linker.asset_original_reference(module_id),
    }
}

/// Return the source file id for one HTML document tree.
fn html_document_file_id(document: &Html) -> FileId {
    document.tree.span(document.document).file
}

/// Return whether one element name matches one HTML local name.
pub(super) fn is_html_element_name(tree: &html::Tree, name: &html::Name, local_name: &str) -> bool {
    matches!(name.namespace, html::Namespace::Html) && tree.strings.get(name.local) == local_name
}

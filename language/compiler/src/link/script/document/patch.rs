use destack_artifact::Html;
use destack_html as html;
use destack_source::{FileId, ModuleEdge, Span};
use destack_workspace::Module;

use crate::{LinkError, LinkResult};

use super::super::ScriptLinker;
use super::super::plan::Plan;
use super::render::{is_html_element_name, render_html_attribute_value};
use crate::link::OutputLocation;

/// Patch one linked document source with resolved references.
pub(super) fn patch_document_source(
    linker: &ScriptLinker<'_>,
    module: &Module,
    module_edges: &[ModuleEdge],
    document: &Html,
    document_location: &OutputLocation,
    plan: &Plan,
    file_id: FileId,
    source: &str,
    associated_stylesheets: &[String],
) -> LinkResult<Option<String>> {
    let document_node = document.tree.get(document.document);
    let mut replacements = Vec::<(Span, String)>::new();

    // collect replacements in tree order
    collect_node_replacements(
        linker,
        module,
        module_edges,
        document,
        document_location,
        plan,
        &document_node.children,
        &mut replacements,
    )?;

    // preserve authored html when stylesheet links can be inserted directly
    if !associated_stylesheets.is_empty()
        && !collect_document_stylesheet_insertions(
            linker,
            document,
            &document_node.children,
            file_id,
            source,
            associated_stylesheets,
            &mut replacements,
        )?
    {
        return Ok(None);
    }

    // apply replacements in source order
    replacements.sort_by_key(|(span, _)| span.start);

    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;

    for (span, value) in replacements {
        if span.file != file_id {
            return Err(LinkError::Internal {
                anchor: (linker.package_id).into(),
                package: linker.package_id,
                message: "html replacement span belongs to the wrong file".to_string(),
            });
        }

        let start = span.start as usize;
        let end = span.end as usize;

        if start < cursor || end > source.len() {
            return Err(LinkError::Internal {
                anchor: (linker.package_id).into(),
                package: linker.package_id,
                message: "html replacement span is invalid".to_string(),
            });
        }

        output.push_str(&source[cursor..start]);
        output.push_str(&value);
        cursor = end;
    }

    output.push_str(&source[cursor..]);

    Ok(Some(output))
}

/// Collect direct stylesheet insertions for one document source.
fn collect_document_stylesheet_insertions(
    linker: &ScriptLinker<'_>,
    document: &Html,
    nodes: &[html::LocalNodeId<html::Content>],
    file_id: FileId,
    source: &str,
    associated_stylesheets: &[String],
    replacements: &mut Vec<(Span, String)>,
) -> LinkResult<bool> {
    let Some(closing_tag_start) =
        find_head_insertion_start(linker, document, nodes, file_id, source)?
    else {
        return Ok(false);
    };
    let Some((indent_start, insertion)) =
        render_head_stylesheet_insertion(source, closing_tag_start, associated_stylesheets)
    else {
        return Ok(false);
    };

    replacements.push((
        Span::new(file_id, indent_start as u32, closing_tag_start as u32),
        insertion,
    ));

    Ok(true)
}

/// Return the insertion position before one authored closing head tag.
fn find_head_insertion_start(
    linker: &ScriptLinker<'_>,
    document: &Html,
    nodes: &[html::LocalNodeId<html::Content>],
    file_id: FileId,
    source: &str,
) -> LinkResult<Option<usize>> {
    for node_id in nodes {
        let html::Content::Element(element) = document.tree.get(*node_id) else {
            continue;
        };

        // insert directly before the authored closing head tag
        if is_html_element_name(&document.tree, &element.name, "head") {
            let span = document.tree.span(*node_id);

            if span.file != file_id {
                return Err(LinkError::Internal {
                    anchor: (linker.package_id).into(),
                    package: linker.package_id,
                    message: "html head span belongs to the wrong file".to_string(),
                });
            }

            if !element.has_authored_end_tag {
                return Ok(None);
            }

            let start = span.start as usize;
            let end = span.end as usize;
            let Some(relative_start) = source[start..end].rfind("</") else {
                return Err(LinkError::Internal {
                    anchor: (linker.package_id).into(),
                    package: linker.package_id,
                    message: "html head element is missing a closing tag slice".to_string(),
                });
            };

            return Ok(Some(start + relative_start));
        }

        // otherwise keep walking the authored tree
        if let Some(insertion_start) =
            find_head_insertion_start(linker, document, &element.children, file_id, source)?
        {
            return Ok(Some(insertion_start));
        }

        if let Some(fragment_id) = element.content {
            let fragment = document.tree.get(fragment_id);

            if let Some(insertion_start) =
                find_head_insertion_start(linker, document, &fragment.children, file_id, source)?
            {
                return Ok(Some(insertion_start));
            }
        }
    }

    Ok(None)
}

/// Render one preserved-format stylesheet insertion block.
fn render_head_stylesheet_insertion(
    source: &str,
    closing_tag_start: usize,
    associated_stylesheets: &[String],
) -> Option<(usize, String)> {
    let line_start = source[..closing_tag_start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let closing_indent = &source[line_start..closing_tag_start];

    // preserve authored formatting only when the closing tag already starts its own line
    if !closing_indent.chars().all(char::is_whitespace) {
        return None;
    }

    let child_indent = format!("{closing_indent}    ");
    let mut output = String::new();

    // emit one authored style line per associated stylesheet
    for href in associated_stylesheets {
        output.push_str(&child_indent);
        output.push_str("<link rel=\"stylesheet\" href=\"");

        for character in href.chars() {
            if character == '"' {
                output.push_str("&quot;");
            } else {
                output.push(character);
            }
        }

        output.push_str("\">\n");
    }

    output.push_str(closing_indent);

    Some((line_start, output))
}

/// Collect replacements from one HTML node list.
fn collect_node_replacements(
    linker: &ScriptLinker<'_>,
    module: &Module,
    module_edges: &[ModuleEdge],
    document: &Html,
    document_location: &OutputLocation,
    plan: &Plan,
    nodes: &[html::LocalNodeId<html::Content>],
    replacements: &mut Vec<(Span, String)>,
) -> LinkResult<()> {
    for node_id in nodes {
        let html::Content::Element(element) = document.tree.get(*node_id) else {
            continue;
        };

        collect_attribute_replacements(
            linker,
            module,
            module_edges,
            document,
            document_location,
            plan,
            &element.attributes,
            replacements,
        )?;
        collect_node_replacements(
            linker,
            module,
            module_edges,
            document,
            document_location,
            plan,
            &element.children,
            replacements,
        )?;

        if let Some(fragment_id) = element.content {
            let fragment = document.tree.get(fragment_id);

            collect_node_replacements(
                linker,
                module,
                module_edges,
                document,
                document_location,
                plan,
                &fragment.children,
                replacements,
            )?;
        }
    }

    Ok(())
}

/// Collect replacements from one HTML attribute list.
fn collect_attribute_replacements(
    linker: &ScriptLinker<'_>,
    module: &Module,
    module_edges: &[ModuleEdge],
    document: &Html,
    document_location: &OutputLocation,
    plan: &Plan,
    attributes: &[html::LocalNodeId<html::Attribute>],
    replacements: &mut Vec<(Span, String)>,
) -> LinkResult<()> {
    for attribute_id in attributes {
        let attribute = document.tree.get(*attribute_id);
        let Some(value) = attribute.value.as_ref() else {
            continue;
        };
        let Some(_) = value.resource.as_ref() else {
            continue;
        };
        let rendered = render_html_attribute_value(
            linker,
            module,
            module_edges,
            document_location,
            plan,
            *attribute_id,
            value,
        )?;

        if rendered == value.value {
            continue;
        }

        let Some(span) = document.tree.value_span(*attribute_id) else {
            return Err(LinkError::Internal {
                anchor: (linker.package_id).into(),
                package: linker.package_id,
                message: format!(
                    "html linked attribute is missing a source span in module '{}'",
                    module.uri
                ),
            });
        };

        replacements.push((span, rendered));
    }

    Ok(())
}

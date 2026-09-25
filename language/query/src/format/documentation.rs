use tspp_dir as dir;

use crate::{QueryError, QueryResult};

use super::Formatter;

impl Formatter<'_, '_, '_> {
    /// Format complete documentation as Markdown.
    pub(crate) fn documentation(&self, documentation: &dir::Documentation) -> QueryResult<String> {
        self.render_documentation(documentation, true)
    }

    /// Format callable documentation without parameter rows.
    pub(crate) fn callable_documentation(
        &self,
        documentation: &dir::Documentation,
    ) -> QueryResult<String> {
        self.render_documentation(documentation, false)
    }

    /// Render documentation with or without parameter rows.
    fn render_documentation(
        &self,
        documentation: &dir::Documentation,
        include_parameters: bool,
    ) -> QueryResult<String> {
        let markdown = self.module.strings().get(documentation.markdown);
        let mut type_parameters = Vec::new();
        let mut parameters = Vec::new();
        let mut examples = Vec::new();
        let mut sections = Vec::new();

        // collect every parsed tag
        for tag in &documentation.tags {
            match *tag {
                dir::DocumentationTag::TypeParameter {
                    parameter,
                    markdown,
                } => {
                    let parameter = self.generic_parameter(parameter)?;
                    let documentation = self.module.strings().get(markdown);
                    type_parameters.push((parameter, documentation));
                }
                dir::DocumentationTag::Parameter {
                    parameter,
                    markdown,
                } if include_parameters => {
                    let parameter = self.module.view()?.get(parameter);
                    let parameter = self.module.parameter_name(parameter)?;
                    let documentation = self.module.strings().get(markdown);
                    parameters.push((parameter, documentation));
                }
                dir::DocumentationTag::Parameter { .. } => {}
                dir::DocumentationTag::Example { markdown } => {
                    examples.push(self.module.strings().get(markdown));
                }
                dir::DocumentationTag::Section { markdown } => {
                    sections.push(self.module.strings().get(markdown));
                }
            }
        }

        let mut blocks = Vec::new();
        if !markdown.is_empty() {
            blocks.push(markdown.to_string());
        }

        // format named documentation lists
        for (heading, entries) in [
            ("Type parameters", type_parameters),
            ("Parameters", parameters),
        ] {
            if entries.is_empty() {
                continue;
            }

            let entries = entries
                .into_iter()
                .map(|(name, documentation)| {
                    if documentation.is_empty() {
                        format!("- `{name}`")
                    } else {
                        format!("- `{name}`: {documentation}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            blocks.push(format!("## {heading}\n\n{entries}"));
        }

        // retain authored section lines
        for section in sections {
            blocks.push(section.to_string());
        }

        // retain authored example order and Markdown
        if !examples.is_empty() {
            blocks.push(format!("## Examples\n\n{}", examples.join("\n\n")));
        }

        Ok(blocks.join("\n\n"))
    }

    /// Return documentation for one exact parameter target.
    pub(crate) fn parameter_documentation(
        &self,
        owner: Option<dir::LocalNodeIdAny>,
        parameter: dir::LocalNodeId<dir::Parameter>,
    ) -> QueryResult<Option<String>> {
        let view = self.module.view()?;
        let parameter_documentation = view.get_documentation(parameter);
        let owner_documentation = owner.and_then(|owner| view.get_documentation_any(owner));

        // select the exact parameter tag from its owner
        let mut tags = owner_documentation
            .into_iter()
            .flat_map(|documentation| documentation.tags.iter())
            .filter_map(|tag| match tag {
                dir::DocumentationTag::Parameter {
                    parameter: candidate,
                    markdown,
                } if *candidate == parameter => Some(*markdown),
                _ => None,
            });
        let tag = tags.next();
        if tags.next().is_some() {
            let parameter = parameter.into_global_any(self.module.module_id());

            return Err(QueryError::conflict(format!(
                "parameter documentation tags: {parameter:?}"
            )));
        }

        // require one unambiguous documentation form
        let markdown = match (parameter_documentation, tag) {
            (Some(documentation), None) => self.documentation(documentation)?,
            (None, Some(markdown)) => self.module.strings().get(markdown).to_string(),
            (None, None) => return Ok(None),
            (Some(_), Some(_)) => {
                let parameter = parameter.into_global_any(self.module.module_id());

                return Err(QueryError::conflict(format!(
                    "parameter documentation forms: {parameter:?}"
                )));
            }
        };

        Ok(Some(markdown))
    }
}

use tspp_dir as dir;

use crate::DocResult;

use super::Printer;

impl Printer<'_, '_, '_> {
    /// Format complete documentation as Markdown.
    pub(crate) fn documentation(&self, documentation: &dir::Documentation) -> DocResult<String> {
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
                } => {
                    let parameter = self.module.view().get(parameter);
                    let parameter = self.module.parameter_name(parameter)?;
                    let documentation = self.module.strings().get(markdown);
                    parameters.push((parameter, documentation));
                }
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
}

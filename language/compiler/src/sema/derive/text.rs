use destack_dir as dir;

use crate::sema::derive::{Component, ComponentProjection, Composite, Derivation};
use crate::sema::{CheckState, Origin, Value};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Build `` `Name { a: ${this.a}, b: ${this.b} }` `` at the formatting result.
    pub(super) fn build_text_body(
        &mut self,
        frame: &mut Derivation,
        shape: Composite,
        components: &[Component],
        origin: Origin,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let string = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::String))?;

        // pick the text the shape opens and closes with
        let (open, close) = match shape {
            Composite::Struct(symbol) | Composite::Class(symbol) => (
                format!("{} {{ ", self.symbol_name(symbol)?),
                " }".to_string(),
            ),
            Composite::Newtype(symbol) => {
                (format!("{}(", self.symbol_name(symbol)?), ")".to_string())
            }
            Composite::Tuple => ("(".to_string(), ")".to_string()),
            Composite::Union => {
                return Err(CompilerError::Internal {
                    message: "a union rendered outside its member dispatch".to_owned(),
                });
            }
        };

        // write the text ahead of each component, then the component itself
        let mut chunks = Vec::with_capacity(components.len() + 1);
        let mut spans = Vec::with_capacity(components.len());
        let mut arguments = Vec::with_capacity(components.len());
        let mut text = open;
        for (index, component) in components.iter().enumerate() {
            if index > 0 {
                text.push_str(", ");
            }
            match component.read {
                ComponentProjection::Field {
                    key: dir::StaticKey::Name(name),
                    ..
                } => {
                    text.push_str(self.strings().get(name));
                    text.push_str(": ");
                }
                ComponentProjection::Base => text.push_str("super: "),
                _ => {}
            }
            let cooked = self.strings().intern(&text);
            chunks.push(dir::TemplateChunk {
                cooked: Some(cooked),
                raw: cooked,
            });
            text = String::new();

            // render the component through its own protocol call
            let this = self.build_this(frame)?;
            let read = self.build_component_read(frame, this, frame.receiver, component)?;
            let rendered = self.build_component_call(frame, read, component, Vec::new())?;
            let rendered_type = self.require_node_type(rendered.into_global_any(frame.module))?;
            let argument = self.build_node(
                frame.module,
                frame.span,
                dir::Argument::Positional { value: rendered },
            );
            arguments.push(argument);

            // display the rendered text itself into the span
            let value = Value {
                ty: rendered_type,
                node: None,
                place: None,
                is_fresh: false,
            };
            let key = dir::LanguageItem::Display.member("display").key;
            let Some((_, span)) = self.select_language_protocol_call(
                origin,
                value,
                rendered_type,
                dir::MemberSpace::Instance,
                key,
                dir::LanguageItem::Display,
                &[],
                &[],
                &[],
            )?
            else {
                return Err(CompilerError::Internal {
                    message: "a rendered component outside the display protocol".to_owned(),
                });
            };
            let dir::OperationResolution::One(span) = span.resolution else {
                return Err(CompilerError::Internal {
                    message: "a rendered component displayed over a union".to_owned(),
                });
            };
            frame.calls.push(span.clone());
            spans.push(span);
        }

        // close the text after the last component
        text.push_str(&close);
        let cooked = self.strings().intern(&text);
        chunks.push(dir::TemplateChunk {
            cooked: Some(cooked),
            raw: cooked,
        });

        // join the chunks with the rendered spans
        let Some(build) = self.select_template_join(origin, string)? else {
            return Err(CompilerError::Internal {
                message: "a derived rendering without its join".to_owned(),
            });
        };
        frame.calls.push(build.clone());
        let template = self.build_expression(
            frame,
            dir::Expression::TemplateExpression {
                value: dir::TemplateLiteral::InterpolatedString { chunks, arguments },
            },
            string,
        )?;
        self.commit_decision(
            template.into_global_any(frame.module),
            dir::Decision::Template(Box::new(dir::TemplateDecision { spans, build })),
        )?;

        // hand the text to the caller as an owned value
        let owned = self.build_owned_text(frame, origin, template, string, result)?;

        self.build_block(frame, Vec::new(), Some(owned), result)
    }

    /// Build the owned wrapping one rendered string takes at the formatting result.
    pub(super) fn build_owned_text(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        text: dir::LocalNodeId<dir::Expression>,
        string: dir::GlobalTypeId,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // select the owned constructor the formatting result declares
        let key = dir::LanguageItem::Cow.member("owned").key;
        let receiver = Value {
            ty: result,
            node: None,
            place: None,
            is_fresh: false,
        };
        let Some((_, call)) = self.select_language_protocol_call(
            origin,
            receiver,
            result,
            dir::MemberSpace::Static,
            key,
            dir::LanguageItem::Cow,
            &[],
            &[],
            &[dir::ArgumentSource::Static(string)],
        )?
        else {
            return Err(CompilerError::Internal {
                message: "a formatting result without its owned constructor".to_owned(),
            });
        };

        // call that constructor on the result type over the rendered text
        let receiver = self.build_type_value(frame, result)?;

        self.build_call(frame, receiver, key, &call, vec![text])
    }

    /// Return the declared name of one symbol, empty when anonymous.
    pub(super) fn symbol_name(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<String> {
        Ok(self
            .binding_table(symbol.module_id)?
            .get_symbol(symbol.local_id)
            .name()
            .map(|name| self.strings().get(name).to_string())
            .unwrap_or_default())
    }
}

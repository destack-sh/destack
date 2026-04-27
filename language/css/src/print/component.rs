use super::TokenRenderer;
use super::printer::Printer;
use crate::{
    BlockKind, ComponentFragment, ComponentValue, ComponentValueList, Function, LocalNodeId,
    Number, SimpleBlock, Token, Tree,
};

/// Print one component fragment as canonical CSS source.
pub fn print_component_fragment(tree: &Tree, fragment: LocalNodeId<ComponentFragment>) -> String {
    Printer::new(tree).render_component_value_list(&tree.get(fragment).value)
}

impl<'a> Printer<'a> {
    /// Render one component value list as canonical CSS source.
    pub(crate) fn render_component_value_list(&self, components: &ComponentValueList) -> String {
        let mut source = String::new();

        self.write_component_value_list(&mut source, components);

        source
    }

    /// Render one number token payload as canonical CSS source.
    pub(crate) fn render_number(number: Number) -> String {
        TokenRenderer::render_number_source(number)
    }

    /// Write one component value list into CSS source.
    pub(crate) fn write_component_value_list(
        &self,
        source: &mut String,
        components: &ComponentValueList,
    ) {
        for value in &components.values {
            self.write_component_value(source, value);
        }
    }

    /// Write one component value into CSS source.
    pub(crate) fn write_component_value(&self, source: &mut String, value: &ComponentValue) {
        match value {
            ComponentValue::Token(token) => self.write_token(source, token),
            ComponentValue::Function(function) => self.write_function(source, function),
            ComponentValue::Block(block) => self.write_simple_block(source, block),
        }
    }

    /// Write one function into CSS source.
    pub(crate) fn write_function(&self, source: &mut String, function: &Function) {
        source.push_str(
            &self
                .token_renderer()
                .render_identifier(self.tree.strings.get(function.name).as_ref()),
        );
        source.push('(');
        self.write_component_value_list(source, &function.arguments);
        source.push(')');
    }

    /// Write one simple block into CSS source.
    pub(crate) fn write_simple_block(&self, source: &mut String, block: &SimpleBlock) {
        source.push(match block.kind {
            BlockKind::Parenthesis => '(',
            BlockKind::SquareBracket => '[',
            BlockKind::CurlyBracket => '{',
        });
        self.write_component_value_list(source, &block.value);
        source.push(match block.kind {
            BlockKind::Parenthesis => ')',
            BlockKind::SquareBracket => ']',
            BlockKind::CurlyBracket => '}',
        });
    }

    /// Write one token into CSS source.
    pub(crate) fn write_token(&self, source: &mut String, token: &Token) {
        self.token_renderer().write_token(source, token);
    }

    /// Return one token renderer over this tree.
    pub(crate) fn token_renderer(&self) -> TokenRenderer<'_> {
        TokenRenderer::new(&self.tree.strings)
    }
}

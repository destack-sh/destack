use destack_core::ImmutableStringPool;
use destack_fir::format::FileMarker;
use destack_source::FileType;
use std::convert::Infallible;

use crate::tree::Precedence;
use crate::{
    Annotation, Argument, ArrayElement, Block, CatchClause, Declaration, Declarator,
    DependencyItem, EnumField, Expression, GenericParameter, JsSourceMap, LocalNodeId,
    LocalNodeIdAny, Member, NOOP_JS_SOURCE_MAP, NodeTree, NodeType, Parameter, Pattern,
    PatternField, Property, Statement, SwitchCase, TupleElement, TypeExpression, TypeMember,
};

/// The result type for direct JS printing.
pub type JsPrintResult<T> = Result<T, Infallible>;

/// One printed JS payload.
#[derive(Debug, Clone)]
pub struct PrintedScript {
    /// The printed JS text.
    pub code: String,
    /// The source markers carried by this print pass.
    pub markers: Vec<FileMarker>,
}

/// Print one root list through the direct minified printer.
pub fn print_roots_minified(
    file_type: FileType,
    tree: &NodeTree,
    roots: &[LocalNodeIdAny],
    strings: &ImmutableStringPool,
) -> JsPrintResult<PrintedScript> {
    let mut printer = Printer::new(file_type, tree, roots, strings, &NOOP_JS_SOURCE_MAP);
    printer.print_roots()?;
    Ok(printer.finish())
}

/// Print one root list through the direct minified printer with one source span provider.
pub fn print_roots_minified_with_source_map(
    file_type: FileType,
    tree: &NodeTree,
    roots: &[LocalNodeIdAny],
    strings: &ImmutableStringPool,
    source_map: &dyn JsSourceMap,
) -> JsPrintResult<PrintedScript> {
    let mut printer = Printer::new(file_type, tree, roots, strings, source_map);
    printer.print_roots()?;
    Ok(printer.finish())
}

/// One direct script printer for minified output.
#[derive(Debug)]
pub(crate) struct Printer<'a> {
    /// The lowered JS tree.
    pub(crate) tree: &'a NodeTree,
    /// The root nodes to print.
    pub(crate) roots: &'a [LocalNodeIdAny],
    /// The string pool.
    pub(crate) strings: &'a ImmutableStringPool,
    /// The source span provider.
    pub(crate) source_map: &'a dyn JsSourceMap,
    /// Whether type syntax should be emitted.
    pub(crate) include_types: bool,
    /// Whether annotations should be emitted.
    pub(crate) include_annotations: bool,
    /// The printed module text.
    pub(crate) code: String,
    /// The exact output to source markers.
    pub(crate) markers: Vec<FileMarker>,
}

impl<'a> Printer<'a> {
    /// Return whether one declaration should be elided from plain js output.
    pub(crate) fn declaration_is_elided(&self, declaration: &Declaration) -> bool {
        !self.include_types && declaration.is_type_only()
    }

    /// Return whether one statement should be elided from plain js output.
    pub(crate) fn statement_is_elided(&self, statement: &Statement) -> bool {
        !self.include_types && statement.is_type_only(self.tree)
    }

    /// Return whether one root should be elided from plain js output.
    pub(crate) fn root_is_elided(&self, root_id: LocalNodeIdAny) -> bool {
        if self.include_types {
            return false;
        }

        match root_id.ty {
            NodeType::Declaration => {
                let declaration_id = LocalNodeId::<Declaration>::new(root_id.id);
                let declaration = self.tree.get(declaration_id);
                declaration.is_type_only()
            }
            NodeType::Statement => {
                let statement_id = LocalNodeId::<Statement>::new(root_id.id);
                let statement = self.tree.get(statement_id);
                statement.is_type_only(self.tree)
            }
            NodeType::Expression => {
                let expression_id = LocalNodeId::<Expression>::new(root_id.id);
                let expression = self.tree.get(expression_id);
                expression.is_type_only(self.tree)
            }
            _ => false,
        }
    }

    /// Create one direct script printer.
    pub(crate) fn new(
        file_type: FileType,
        tree: &'a NodeTree,
        roots: &'a [LocalNodeIdAny],
        strings: &'a ImmutableStringPool,
        source_map: &'a dyn JsSourceMap,
    ) -> Self {
        Self {
            tree,
            roots,
            strings,
            source_map,
            include_types: matches!(
                file_type,
                FileType::TypeScript | FileType::TypeScriptXml | FileType::TypeScriptDeclaration
            ),
            include_annotations: matches!(
                file_type,
                FileType::TypeScript | FileType::TypeScriptXml | FileType::TypeScriptDeclaration
            ),
            code: String::new(),
            markers: Vec::new(),
        }
    }

    /// Finish the current printed module.
    pub(crate) fn finish(self) -> PrintedScript {
        PrintedScript {
            code: self.code,
            markers: self.markers,
        }
    }

    /// Print all root nodes.
    pub(crate) fn print_roots(&mut self) -> JsPrintResult<()> {
        for (index, root_id) in self.roots.iter().copied().enumerate() {
            if self.root_is_elided(root_id) {
                continue;
            }

            let has_next = self
                .roots
                .iter()
                .copied()
                .skip(index + 1)
                .any(|next_root| !self.root_is_elided(next_root));

            self.print_root(root_id)?;

            if self.root_needs_separator(root_id, has_next) {
                self.write_punct(";");
            }
        }

        Ok(())
    }

    /// Return whether one root needs one trailing separator.
    pub(crate) fn root_needs_separator(&self, root_id: LocalNodeIdAny, has_next: bool) -> bool {
        if self.root_is_elided(root_id) {
            return false;
        }

        if root_id.ty == NodeType::Statement {
            let statement_id = LocalNodeId::<Statement>::new(root_id.id);
            let statement = self.tree.get(statement_id);

            if self.statement_is_elided(statement) {
                return false;
            }
            return has_next || statement.needs_semicolon();
        }

        has_next
    }

    /// Print one root node.
    pub(crate) fn print_root(&mut self, root_id: LocalNodeIdAny) -> JsPrintResult<()> {
        match root_id.ty {
            NodeType::Block => self.print_block_id(LocalNodeId::new(root_id.id)),
            NodeType::CatchClause => self.print_catch_clause_id(LocalNodeId::new(root_id.id)),
            NodeType::Statement => self.print_statement_id(LocalNodeId::new(root_id.id)),
            NodeType::Expression => self.print_expression_id(LocalNodeId::new(root_id.id)),
            NodeType::ArrayElement => self.print_array_element_id(LocalNodeId::new(root_id.id)),
            NodeType::Declaration => self.print_declaration_id(LocalNodeId::new(root_id.id)),
            NodeType::Declarator => self.print_declarator_id(LocalNodeId::new(root_id.id)),
            NodeType::Property => self.print_property_id(LocalNodeId::new(root_id.id)),
            NodeType::Member => self.print_member_id(LocalNodeId::new(root_id.id)),
            NodeType::TypeExpression => self.print_type_id(LocalNodeId::new(root_id.id)),
            NodeType::TupleElement => self.print_tuple_element_id(LocalNodeId::new(root_id.id)),
            NodeType::TypeMember => self.print_type_member_id(LocalNodeId::new(root_id.id)),
            NodeType::EnumField => self.print_enum_field_id(LocalNodeId::new(root_id.id)),
            NodeType::DependencyItem => self.print_dependency_item_id(LocalNodeId::new(root_id.id)),
            NodeType::SwitchCase => self.print_switch_case_id(LocalNodeId::new(root_id.id)),
            NodeType::Pattern => self.print_pattern_id(LocalNodeId::new(root_id.id)),
            NodeType::PatternField => self.print_pattern_field_id(LocalNodeId::new(root_id.id)),
            NodeType::GenericParameter => {
                self.print_generic_parameter_id(LocalNodeId::new(root_id.id))
            }
            NodeType::Parameter => self.print_parameter_id(LocalNodeId::new(root_id.id)),
            NodeType::Argument => self.print_argument_id(LocalNodeId::new(root_id.id)),
            NodeType::Annotation => self.print_annotation_id(LocalNodeId::new(root_id.id)),
        }
    }

    /// Print one catch clause id.
    pub(crate) fn print_catch_clause_id(
        &mut self,
        catch_clause_id: LocalNodeId<CatchClause>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(catch_clause_id.id, |this| {
            let tree = this.tree;
            let catch_clause = tree.get(catch_clause_id);

            this.write_keyword(crate::Keyword::Catch);

            if let Some(pattern) = catch_clause.pattern {
                this.write_punct("(");
                this.print_pattern_id(pattern)?;
                this.write_punct(")");
            }

            this.print_block_id(catch_clause.body)
        })
    }

    /// Print one statement list inside one block.
    pub(crate) fn print_statement_list(
        &mut self,
        statements: &[LocalNodeId<Statement>],
    ) -> JsPrintResult<()> {
        for (index, statement_id) in statements.iter().copied().enumerate() {
            let statement = self.tree.get(statement_id);
            if self.statement_is_elided(statement) {
                continue;
            }

            self.print_statement_id(statement_id)?;

            let has_next = statements
                .iter()
                .copied()
                .skip(index + 1)
                .any(|next_statement_id| {
                    let next_statement = self.tree.get(next_statement_id);
                    !self.statement_is_elided(next_statement)
                });

            if has_next || statement.needs_semicolon() {
                self.write_punct(";");
            }
        }

        Ok(())
    }

    /// Print one class or interface member list.
    pub(crate) fn print_member_list(
        &mut self,
        members: &[LocalNodeId<Member>],
    ) -> JsPrintResult<()> {
        for (index, member_id) in members.iter().enumerate() {
            self.print_member_id(*member_id)?;

            if index + 1 < members.len() {
                self.write_punct(";");
            }
        }

        Ok(())
    }

    /// Print one enum field list.
    pub(crate) fn print_enum_field_list(
        &mut self,
        fields: &[LocalNodeId<EnumField>],
    ) -> JsPrintResult<()> {
        for (index, field_id) in fields.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_enum_field_id(*field_id)?;
        }

        Ok(())
    }

    /// Print one comma-separated expression list.
    pub(crate) fn print_expression_list(
        &mut self,
        expressions: &[LocalNodeId<Expression>],
    ) -> JsPrintResult<()> {
        for (index, expression_id) in expressions.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_expression_id(*expression_id)?;
        }

        Ok(())
    }

    /// Print one comma-separated array element list.
    pub(crate) fn print_array_element_list(
        &mut self,
        elements: &[LocalNodeId<ArrayElement>],
    ) -> JsPrintResult<()> {
        for (index, element_id) in elements.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_array_element_id(*element_id)?;
        }

        Ok(())
    }

    /// Print one comma-separated argument list.
    pub(crate) fn print_argument_list(
        &mut self,
        arguments: &[LocalNodeId<Argument>],
    ) -> JsPrintResult<()> {
        let mut wrote_argument = false;

        for argument_id in arguments {
            if !self.should_print_argument(*argument_id) {
                continue;
            }

            if wrote_argument {
                self.write_punct(",");
            }

            self.print_argument_id(*argument_id)?;
            wrote_argument = true;
        }

        Ok(())
    }

    /// Print one comma-separated parameter list.
    pub(crate) fn print_parameter_list(
        &mut self,
        parameters: &[LocalNodeId<Parameter>],
    ) -> JsPrintResult<()> {
        for (index, parameter_id) in parameters.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_parameter_id(*parameter_id)?;
        }

        Ok(())
    }

    /// Print one comma-separated dependency item list.
    pub(crate) fn print_dependency_item_list(
        &mut self,
        items: &[LocalNodeId<DependencyItem>],
    ) -> JsPrintResult<()> {
        for (index, item_id) in items.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_dependency_item_id(*item_id)?;
        }

        Ok(())
    }

    /// Print one comma-separated type list.
    pub(crate) fn print_type_list(
        &mut self,
        types: &[LocalNodeId<TypeExpression>],
    ) -> JsPrintResult<()> {
        for (index, type_id) in types.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_type_id(*type_id)?;
        }

        Ok(())
    }

    /// Print one generic type argument list.
    pub(crate) fn print_type_arguments(
        &mut self,
        generic_arguments: &[LocalNodeId<TypeExpression>],
    ) -> JsPrintResult<()> {
        self.write_punct("<");
        self.print_type_list(generic_arguments)?;
        self.write_punct(">");
        Ok(())
    }

    /// Print one object literal property list.
    pub(crate) fn print_property_list(
        &mut self,
        properties: &[LocalNodeId<Property>],
    ) -> JsPrintResult<()> {
        for (index, property_id) in properties.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_property_id(*property_id)?;
        }

        Ok(())
    }

    /// Print one object type member list.
    pub(crate) fn print_type_member_list(
        &mut self,
        properties: &[LocalNodeId<TypeMember>],
    ) -> JsPrintResult<()> {
        for (index, property_id) in properties.iter().enumerate() {
            if index > 0 {
                self.write_punct(";");
            }

            self.print_type_member_id(*property_id)?;
        }

        Ok(())
    }

    /// Print one comma-separated array pattern field list.
    pub(crate) fn print_pattern_array_field_list(
        &mut self,
        fields: &[LocalNodeId<PatternField>],
    ) -> JsPrintResult<()> {
        for (index, field_id) in fields.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_pattern_field_id(*field_id)?;
        }

        Ok(())
    }

    /// Print one pattern field list.
    pub(crate) fn print_pattern_field_list(
        &mut self,
        fields: &[LocalNodeId<PatternField>],
    ) -> JsPrintResult<()> {
        for (index, field_id) in fields.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_pattern_field_id(*field_id)?;
        }

        Ok(())
    }

    /// Print one node with source markers.
    pub(crate) fn print_with_node_markers(
        &mut self,
        node_id: u32,
        print: impl FnOnce(&mut Self) -> JsPrintResult<()>,
    ) -> JsPrintResult<()> {
        let source_span = self.source_span(node_id);

        if let Some(source_span) = source_span {
            self.mark_source(source_span.start);
        }

        print(self)?;

        if let Some(source_span) = source_span {
            self.mark_source(source_span.end);
        }

        Ok(())
    }

    /// Print one block id.
    pub(crate) fn print_block_id(&mut self, block_id: LocalNodeId<Block>) -> JsPrintResult<()> {
        self.print_with_node_markers(block_id.id, |this| {
            let tree = this.tree;
            let block = tree.get(block_id);
            this.write_punct("{");
            this.print_statement_list(&block.statements)?;
            this.write_punct("}");
            Ok(())
        })
    }

    /// Print one statement id.
    pub(crate) fn print_statement_id(
        &mut self,
        statement_id: LocalNodeId<Statement>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(statement_id.id, |this| {
            let tree = this.tree;
            let statement = tree.get(statement_id);
            this.print_statement(statement_id, statement)
        })
    }

    /// Print one expression id.
    pub(crate) fn print_expression_id(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> JsPrintResult<()> {
        self.print_expression_id_with_precedence(expression_id, Precedence::Lowest)
    }

    /// Print one expression id with one parent precedence.
    pub(crate) fn print_expression_id_with_precedence(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        parent_precedence: Precedence,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(expression_id.id, |this| {
            let tree = this.tree;
            let expression = tree.get(expression_id);
            this.print_expression(expression_id, expression, parent_precedence)
        })
    }

    /// Print one declaration id.
    pub(crate) fn print_declaration_id(
        &mut self,
        declaration_id: LocalNodeId<Declaration>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(declaration_id.id, |this| {
            let tree = this.tree;
            let declaration = tree.get(declaration_id);
            this.print_declaration(declaration, declaration_id)
        })
    }

    /// Print one declarator id.
    pub(crate) fn print_declarator_id(
        &mut self,
        declarator_id: LocalNodeId<Declarator>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(declarator_id.id, |this| {
            let tree = this.tree;
            let declarator = tree.get(declarator_id);
            this.print_declarator(declarator)
        })
    }

    /// Print one property id.
    pub(crate) fn print_property_id(
        &mut self,
        property_id: LocalNodeId<Property>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(property_id.id, |this| {
            let tree = this.tree;
            let property = tree.get(property_id);
            this.print_property(property)
        })
    }

    /// Print one member id.
    pub(crate) fn print_member_id(&mut self, member_id: LocalNodeId<Member>) -> JsPrintResult<()> {
        self.print_with_node_markers(member_id.id, |this| {
            let tree = this.tree;
            let member = tree.get(member_id);
            this.print_member(member)
        })
    }

    /// Print one type id.
    pub(crate) fn print_type_id(
        &mut self,
        type_id: LocalNodeId<TypeExpression>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(type_id.id, |this| {
            let tree = this.tree;
            let ty = tree.get(type_id);
            this.print_type(ty)
        })
    }

    /// Print one tuple element id.
    pub(crate) fn print_tuple_element_id(
        &mut self,
        tuple_element_id: LocalNodeId<TupleElement>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(tuple_element_id.id, |this| {
            let tree = this.tree;
            let tuple_element = tree.get(tuple_element_id);
            this.print_tuple_element(tuple_element)
        })
    }

    /// Print one type member id.
    pub(crate) fn print_type_member_id(
        &mut self,
        type_member_id: LocalNodeId<TypeMember>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(type_member_id.id, |this| {
            let tree = this.tree;
            let type_member = tree.get(type_member_id);
            this.print_type_member(type_member)
        })
    }

    /// Print one enum field id.
    pub(crate) fn print_enum_field_id(
        &mut self,
        enum_field_id: LocalNodeId<EnumField>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(enum_field_id.id, |this| {
            let tree = this.tree;
            let enum_field = tree.get(enum_field_id);
            this.print_enum_field(enum_field)
        })
    }

    /// Print one dependency item id.
    pub(crate) fn print_dependency_item_id(
        &mut self,
        item_id: LocalNodeId<DependencyItem>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(item_id.id, |this| {
            let tree = this.tree;
            let item = tree.get(item_id);
            this.print_dependency_item(item_id, item)
        })
    }

    /// Print one switch case id.
    pub(crate) fn print_switch_case_id(
        &mut self,
        switch_case_id: LocalNodeId<SwitchCase>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(switch_case_id.id, |this| {
            let tree = this.tree;
            let switch_case = tree.get(switch_case_id);

            if let Some(value) = switch_case.value {
                this.write_keyword(crate::Keyword::Case);
                this.print_expression_id(value)?;
                this.write_punct(":");
            } else {
                this.write_keyword(crate::Keyword::Default);
                this.write_punct(":");
            }

            let body = tree.get(switch_case.body);
            this.print_statement_list(&body.statements)
        })
    }

    /// Print one pattern id.
    pub(crate) fn print_pattern_id(
        &mut self,
        pattern_id: LocalNodeId<Pattern>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(pattern_id.id, |this| {
            let tree = this.tree;
            let pattern = tree.get(pattern_id);
            this.print_pattern(pattern)
        })
    }

    /// Print one pattern field id.
    pub(crate) fn print_pattern_field_id(
        &mut self,
        pattern_field_id: LocalNodeId<PatternField>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(pattern_field_id.id, |this| {
            let tree = this.tree;
            let pattern_field = tree.get(pattern_field_id);
            this.print_pattern_field(pattern_field)
        })
    }

    /// Print one array element id.
    pub(crate) fn print_array_element_id(
        &mut self,
        array_element_id: LocalNodeId<ArrayElement>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(array_element_id.id, |this| {
            let tree = this.tree;
            let array_element = tree.get(array_element_id);
            this.print_array_element(array_element)
        })
    }

    /// Print one parameter id.
    pub(crate) fn print_parameter_id(
        &mut self,
        parameter_id: LocalNodeId<Parameter>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(parameter_id.id, |this| {
            let tree = this.tree;
            let parameter = tree.get(parameter_id);
            this.print_parameter(parameter)
        })
    }

    /// Print one generic parameter id.
    pub(crate) fn print_generic_parameter_id(
        &mut self,
        parameter_id: LocalNodeId<GenericParameter>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(parameter_id.id, |this| {
            let tree = this.tree;
            let parameter = tree.get(parameter_id);
            this.print_type_parameter(parameter)
        })
    }

    /// Print one argument id.
    pub(crate) fn print_argument_id(
        &mut self,
        argument_id: LocalNodeId<Argument>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(argument_id.id, |this| {
            let tree = this.tree;
            let argument = tree.get(argument_id);
            this.print_argument(argument)
        })
    }

    /// Return whether one argument contributes real output text.
    pub(crate) fn should_print_argument(&self, argument_id: LocalNodeId<Argument>) -> bool {
        let argument = self.tree.get(argument_id);

        match argument {
            // parser placeholders should never leak into emitted call syntax
            Argument::Positional { value } => self.expression_emits_code(*value),
            Argument::Spread { .. } => true,
        }
    }

    /// Return whether one expression produces output in the direct printer.
    pub(crate) fn expression_emits_code(&self, expression_id: LocalNodeId<Expression>) -> bool {
        match self.tree.get(expression_id) {
            Expression::Parenthesized { expression } => self.expression_emits_code(*expression),
            Expression::SequenceExpression { expressions } => expressions
                .iter()
                .copied()
                .any(|expression| self.expression_emits_code(expression)),
            Expression::Missing | Expression::Stub | Expression::Error => false,
            _ => true,
        }
    }

    /// Print one annotation id.
    pub(crate) fn print_annotation_id(
        &mut self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> JsPrintResult<()> {
        self.print_with_node_markers(annotation_id.id, |this| {
            if !this.include_annotations {
                return Ok(());
            }

            let tree = this.tree;
            let annotation = tree.get(annotation_id);
            this.print_annotation(annotation)
        })
    }
}

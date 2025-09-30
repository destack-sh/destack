#![allow(unused_variables)]

use crate::{
    Annotation, Argument, ArrayLiteral, Blank, Block, Break, Call, Cast, Coalesce, Comment, Continue, Decorator, Defer, Doc, Enum, EnumField, Expression, FieldLiteral, For, Function, If, Implement, Index, Let, Loop, Match, MatchCase, Module, NodeId, NodeTree, NodeType, Parameter, Pattern, PatternField, RangeLiteral, Return, ScalarLiteral, Struct, StructField, StructLiteral, Tag, Trait, Try, Tuple, TupleField, TupleLiteral, Type, TypeLiteral, Union, UnionField, Use, UseClause, UseItem, While, With, WithClause, walk_annotation, walk_argument, walk_array_literal, walk_blank, walk_block, walk_break, walk_call, walk_cast, walk_coalesce, walk_comment, walk_continue, walk_decorator, walk_defer, walk_doc, walk_enum, walk_enum_field, walk_expression, walk_field_literal, walk_for, walk_function, walk_if, walk_implement, walk_index, walk_let, walk_loop, walk_match, walk_match_case, walk_module, walk_parameter, walk_pattern, walk_pattern_field, walk_range_literal, walk_return, walk_scalar_literal, walk_struct, walk_struct_field, walk_struct_literal, walk_tag, walk_trait, walk_try, walk_tuple, walk_tuple_field, walk_tuple_literal, walk_type, walk_type_literal, walk_union, walk_union_field, walk_use, walk_use_clause, walk_use_item, walk_while, walk_with, walk_with_clause
};

/// A NodeVisitor is a visitor for the AST.
pub trait NodeVisitor {
    #[inline]
    fn visit_any(&mut self, tree: &NodeTree, ty: NodeType, id: u32) {
        // nothing to do
    }

    // ------------------------------------------------------------
    // Groupings
    // ------------------------------------------------------------

    /// Visit an Expression.
    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Expression>,
        expression: &Expression,
    ) {
        walk_expression(self, tree, id, expression);
    }

    /// Visit a Block.
    fn visit_block(&mut self, tree: &NodeTree, id: NodeId<Block>, block: &Block) {
        walk_block(self, tree, id, block);
    }

    // ------------------------------------------------------------
    // Declarations
    // ------------------------------------------------------------

    /// Visit a Module.
    fn visit_module(&mut self, tree: &NodeTree, id: NodeId<Module>, module: &Module) {
        walk_module(self, tree, id, module);
    }

    /// Visit a Struct.
    fn visit_struct(&mut self, tree: &NodeTree, id: NodeId<Struct>, struct_node: &Struct) {
        walk_struct(self, tree, id, struct_node);
    }

    /// Visit a StructField.
    fn visit_struct_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<StructField>,
        struct_field: &StructField,
    ) {
        walk_struct_field(self, tree, id, struct_field);
    }

    /// Visit an Enum.
    fn visit_enum(&mut self, tree: &NodeTree, id: NodeId<Enum>, enum_node: &Enum) {
        walk_enum(self, tree, id, enum_node);
    }

    /// Visit an EnumField.
    fn visit_enum_field(&mut self, tree: &NodeTree, id: NodeId<EnumField>, enum_field: &EnumField) {
        walk_enum_field(self, tree, id, enum_field);
    }

    /// Visit a Union.
    fn visit_union(&mut self, tree: &NodeTree, id: NodeId<Union>, union_node: &Union) {
        walk_union(self, tree, id, union_node);
    }

    /// Visit a UnionField.
    fn visit_union_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<UnionField>,
        union_field: &UnionField,
    ) {
        walk_union_field(self, tree, id, union_field);
    }

    /// Visit a Trait.
    fn visit_trait(&mut self, tree: &NodeTree, id: NodeId<Trait>, trait_node: &Trait) {
        walk_trait(self, tree, id, trait_node);
    }

    /// Visit an Implement.
    fn visit_implement(&mut self, tree: &NodeTree, id: NodeId<Implement>, implement: &Implement) {
        walk_implement(self, tree, id, implement);
    }

    /// Visit a Type.
    fn visit_type(&mut self, tree: &NodeTree, id: NodeId<Type>, type_node: &Type) {
        walk_type(self, tree, id, type_node);
    }

    /// Visit a Tuple.
    fn visit_tuple(&mut self, tree: &NodeTree, id: NodeId<Tuple>, tuple: &Tuple) {
        walk_tuple(self, tree, id, tuple);
    }

    /// Visit a TupleField.
    fn visit_tuple_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<TupleField>,
        tuple_field: &TupleField,
    ) {
        walk_tuple_field(self, tree, id, tuple_field);
    }

    /// Visit a Function.
    fn visit_function(&mut self, tree: &NodeTree, id: NodeId<Function>, function: &Function) {
        walk_function(self, tree, id, function);
    }

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------

    /// Visit a With.
    fn visit_with(&mut self, tree: &NodeTree, id: NodeId<With>, with: &With) {
        walk_with(self, tree, id, with);
    }

    /// Visit a WithClause.
    fn visit_with_clause(
        &mut self,
        tree: &NodeTree,
        id: NodeId<WithClause>,
        with_clause: &WithClause,
    ) {
        walk_with_clause(self, tree, id, with_clause);
    }

    /// Visit a Use.
    fn visit_use(&mut self, tree: &NodeTree, id: NodeId<Use>, use_node: &Use) {
        walk_use(self, tree, id, use_node);
    }

    /// Visit a UseClause.
    fn visit_use_clause(&mut self, tree: &NodeTree, id: NodeId<UseClause>, use_clause: &UseClause) {
        walk_use_clause(self, tree, id, use_clause);
    }

    /// Visit a UseItem.
    fn visit_use_item(&mut self, tree: &NodeTree, id: NodeId<UseItem>, use_item: &UseItem) {
        walk_use_item(self, tree, id, use_item);
    }

    // ------------------------------------------------------------
    // Control
    // ------------------------------------------------------------

    /// Visit an If.
    fn visit_if(&mut self, tree: &NodeTree, id: NodeId<If>, if_node: &If) {
        walk_if(self, tree, id, if_node);
    }

    /// Visit a While.
    fn visit_while(&mut self, tree: &NodeTree, id: NodeId<While>, while_node: &While) {
        walk_while(self, tree, id, while_node);
    }

    /// Visit a For.
    fn visit_for(&mut self, tree: &NodeTree, id: NodeId<For>, for_node: &For) {
        walk_for(self, tree, id, for_node);
    }

    /// Visit a Loop.
    fn visit_loop(&mut self, tree: &NodeTree, id: NodeId<Loop>, loop_node: &Loop) {
        walk_loop(self, tree, id, loop_node);
    }

    /// Visit a Break.
    fn visit_break(&mut self, tree: &NodeTree, id: NodeId<Break>, break_node: &Break) {
        walk_break(self, tree, id, break_node);
    }

    /// Visit a Continue.
    fn visit_continue(&mut self, tree: &NodeTree, id: NodeId<Continue>, continue_node: &Continue) {
        walk_continue(self, tree, id, continue_node);
    }

    /// Visit a Defer.
    fn visit_defer(&mut self, tree: &NodeTree, id: NodeId<Defer>, defer: &Defer) {
        walk_defer(self, tree, id, defer);
    }

    /// Visit a Return.
    fn visit_return(&mut self, tree: &NodeTree, id: NodeId<Return>, return_node: &Return) {
        walk_return(self, tree, id, return_node);
    }

    /// Visit a Try.
    fn visit_try(&mut self, tree: &NodeTree, id: NodeId<Try>, try_node: &Try) {
        walk_try(self, tree, id, try_node);
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    /// Visit a Let.
    fn visit_let(&mut self, tree: &NodeTree, id: NodeId<Let>, let_node: &Let) {
        walk_let(self, tree, id, let_node);
    }

    /// Visit a Parameter.
    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, parameter: &Parameter) {
        walk_parameter(self, tree, id, parameter);
    }

    /// Visit an Argument.
    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, argument: &Argument) {
        walk_argument(self, tree, id, argument);
    }

    // ------------------------------------------------------------
    // Literals
    // ------------------------------------------------------------

    /// Visit a ScalarLiteral.
    fn visit_scalar_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<ScalarLiteral>,
        scalar_literal: &ScalarLiteral,
    ) {
        walk_scalar_literal(self, tree, id, scalar_literal);
    }

    /// Visit a TypeLiteral.
    fn visit_type_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<TypeLiteral>,
        type_literal: &TypeLiteral,
    ) {
        walk_type_literal(self, tree, id, type_literal);
    }

    /// Visit a RangeLiteral.
    fn visit_range_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<RangeLiteral>,
        range_literal: &RangeLiteral,
    ) {
        walk_range_literal(self, tree, id, range_literal);
    }

    /// Visit an ArrayLiteral.
    fn visit_array_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<ArrayLiteral>,
        array_literal: &ArrayLiteral,
    ) {
        walk_array_literal(self, tree, id, array_literal);
    }

    /// Visit a TupleLiteral.
    fn visit_tuple_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<TupleLiteral>,
        tuple_literal: &TupleLiteral,
    ) {
        walk_tuple_literal(self, tree, id, tuple_literal);
    }

    /// Visit a StructLiteral.
    fn visit_struct_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<StructLiteral>,
        struct_literal: &StructLiteral,
    ) {
        walk_struct_literal(self, tree, id, struct_literal);
    }

    /// Visit a FieldLiteral.
    fn visit_field_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<FieldLiteral>,
        field_literal: &FieldLiteral,
    ) {
        walk_field_literal(self, tree, id, field_literal);
    }

    // ------------------------------------------------------------
    // Calls
    // ------------------------------------------------------------

    /// Visit an Index.
    fn visit_index(&mut self, tree: &NodeTree, id: NodeId<Index>, index: &Index) {
        walk_index(self, tree, id, index);
    }

    /// Visit a Call.
    fn visit_call(&mut self, tree: &NodeTree, id: NodeId<Call>, call: &Call) {
        walk_call(self, tree, id, call);
    }

    /// Visit a Cast.
    fn visit_cast(&mut self, tree: &NodeTree, id: NodeId<Cast>, cast: &Cast) {
        walk_cast(self, tree, id, cast);
    }

    /// Visit a Coalesce.
    fn visit_coalesce(&mut self, tree: &NodeTree, id: NodeId<Coalesce>, coalesce: &Coalesce) {
        walk_coalesce(self, tree, id, coalesce);
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    /// Visit a Match.
    fn visit_match(&mut self, tree: &NodeTree, id: NodeId<Match>, match_node: &Match) {
        walk_match(self, tree, id, match_node);
    }

    /// Visit a MatchCase.
    fn visit_match_case(&mut self, tree: &NodeTree, id: NodeId<MatchCase>, match_case: &MatchCase) {
        walk_match_case(self, tree, id, match_case);
    }

    /// Visit a Pattern.
    fn visit_pattern(&mut self, tree: &NodeTree, id: NodeId<Pattern>, pattern: &Pattern) {
        walk_pattern(self, tree, id, pattern);
    }

    /// Visit a PatternField.
    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        walk_pattern_field(self, tree, id, pattern_field);
    }

    // ------------------------------------------------------------
    // Annotations
    // ------------------------------------------------------------

    /// Visit an Annotation.
    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Annotation>,
        annotation: &Annotation,
    ) {
        walk_annotation(self, tree, id, annotation);
    }

    /// Visit a Blank.
    fn visit_blank(&mut self, tree: &NodeTree, id: NodeId<Blank>, blank: &Blank) {
        walk_blank(self, tree, id, blank);
    }

    /// Visit a Doc.
    fn visit_doc(&mut self, tree: &NodeTree, id: NodeId<Doc>, doc: &Doc) {
        walk_doc(self, tree, id, doc);
    }

    /// Visit a Comment.
    fn visit_comment(&mut self, tree: &NodeTree, id: NodeId<Comment>, comment: &Comment) {
        walk_comment(self, tree, id, comment);
    }

    /// Visit a Tag.
    fn visit_tag(&mut self, tree: &NodeTree, id: NodeId<Tag>, tag: &Tag) {
        walk_tag(self, tree, id, tag);
    }

    /// Visit a Decorator.
    fn visit_decorator(&mut self, tree: &NodeTree, id: NodeId<Decorator>, decorator: &Decorator) {
        walk_decorator(self, tree, id, decorator);
    }
}

/// A CapturingNodeVisitor is a visitor that collects the nodes visited (without walking further).
#[derive(Debug, Clone, Default)]
pub struct CapturingNodeVisitor {
    visited: Vec<u32>,
}

impl CapturingNodeVisitor {
    pub fn new() -> Self {
        Self {
            visited: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.visited.clear();
    }

    pub fn visited(&self) -> &[u32] {
        &self.visited
    }
}

impl NodeVisitor for CapturingNodeVisitor {
    fn visit_any(&mut self, tree: &NodeTree, ty: NodeType, id: u32) {
        self.visited.push(id);
    }

    // ------------------------------------------------------------
    // Groupings
    // ------------------------------------------------------------

    fn visit_block(&mut self, tree: &NodeTree, id: NodeId<Block>, block: &Block) {
        self.visit_any(tree, NodeType::Block, id.id);
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Expression>,
        expression: &Expression,
    ) {
        self.visit_any(tree, NodeType::Expression, id.id);
    }

    // ------------------------------------------------------------
    // Declarations
    // ------------------------------------------------------------

    fn visit_module(&mut self, tree: &NodeTree, id: NodeId<Module>, module: &Module) {
        self.visit_any(tree, NodeType::Module, id.id);
    }

    fn visit_struct(&mut self, tree: &NodeTree, id: NodeId<Struct>, struct_node: &Struct) {
        self.visit_any(tree, NodeType::Struct, id.id);
    }

    fn visit_struct_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<StructField>,
        struct_field: &StructField,
    ) {
        self.visit_any(tree, NodeType::StructField, id.id);
    }

    fn visit_enum(&mut self, tree: &NodeTree, id: NodeId<Enum>, enum_node: &Enum) {
        self.visit_any(tree, NodeType::Enum, id.id);
    }

    fn visit_enum_field(&mut self, tree: &NodeTree, id: NodeId<EnumField>, enum_field: &EnumField) {
        self.visit_any(tree, NodeType::EnumField, id.id);
    }

    fn visit_union(&mut self, tree: &NodeTree, id: NodeId<Union>, union_node: &Union) {
        self.visit_any(tree, NodeType::Union, id.id);
    }

    fn visit_union_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<UnionField>,
        union_field: &UnionField,
    ) {
        self.visit_any(tree, NodeType::UnionField, id.id);
    }

    fn visit_trait(&mut self, tree: &NodeTree, id: NodeId<Trait>, trait_node: &Trait) {
        self.visit_any(tree, NodeType::Trait, id.id);
    }

    fn visit_implement(&mut self, tree: &NodeTree, id: NodeId<Implement>, implement: &Implement) {
        self.visit_any(tree, NodeType::Implement, id.id);
    }

    fn visit_type(&mut self, tree: &NodeTree, id: NodeId<Type>, type_node: &Type) {
        self.visit_any(tree, NodeType::Type, id.id);
    }

    fn visit_tuple(&mut self, tree: &NodeTree, id: NodeId<Tuple>, tuple: &Tuple) {
        self.visit_any(tree, NodeType::Tuple, id.id);
    }

    fn visit_tuple_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<TupleField>,
        tuple_field: &TupleField,
    ) {
        self.visit_any(tree, NodeType::TupleField, id.id);
    }

    fn visit_function(&mut self, tree: &NodeTree, id: NodeId<Function>, function: &Function) {
        self.visit_any(tree, NodeType::Function, id.id);
    }

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------

    fn visit_with(&mut self, tree: &NodeTree, id: NodeId<With>, with: &With) {
        self.visit_any(tree, NodeType::With, id.id);
    }

    fn visit_with_clause(
        &mut self,
        tree: &NodeTree,
        id: NodeId<WithClause>,
        with_clause: &WithClause,
    ) {
        self.visit_any(tree, NodeType::WithClause, id.id);
    }

    fn visit_use(&mut self, tree: &NodeTree, id: NodeId<Use>, use_node: &Use) {
        self.visit_any(tree, NodeType::Use, id.id);
    }

    fn visit_use_clause(&mut self, tree: &NodeTree, id: NodeId<UseClause>, use_clause: &UseClause) {
        self.visit_any(tree, NodeType::UseClause, id.id);
    }

    fn visit_use_item(&mut self, tree: &NodeTree, id: NodeId<UseItem>, use_item: &UseItem) {
        self.visit_any(tree, NodeType::UseItem, id.id);
    }

    // ------------------------------------------------------------
    // Control
    // ------------------------------------------------------------

    fn visit_if(&mut self, tree: &NodeTree, id: NodeId<If>, if_node: &If) {
        self.visit_any(tree, NodeType::If, id.id);
    }

    fn visit_while(&mut self, tree: &NodeTree, id: NodeId<While>, while_node: &While) {
        self.visit_any(tree, NodeType::While, id.id);
    }

    fn visit_for(&mut self, tree: &NodeTree, id: NodeId<For>, for_node: &For) {
        self.visit_any(tree, NodeType::For, id.id);
    }

    fn visit_loop(&mut self, tree: &NodeTree, id: NodeId<Loop>, loop_node: &Loop) {
        self.visit_any(tree, NodeType::Loop, id.id);
    }

    fn visit_break(&mut self, tree: &NodeTree, id: NodeId<Break>, break_node: &Break) {
        self.visit_any(tree, NodeType::Break, id.id);
    }

    fn visit_continue(&mut self, tree: &NodeTree, id: NodeId<Continue>, continue_node: &Continue) {
        self.visit_any(tree, NodeType::Continue, id.id);
    }

    fn visit_defer(&mut self, tree: &NodeTree, id: NodeId<Defer>, defer: &Defer) {
        self.visit_any(tree, NodeType::Defer, id.id);
    }

    fn visit_return(&mut self, tree: &NodeTree, id: NodeId<Return>, return_node: &Return) {
        self.visit_any(tree, NodeType::Return, id.id);
    }

    fn visit_try(&mut self, tree: &NodeTree, id: NodeId<Try>, try_node: &Try) {
        self.visit_any(tree, NodeType::Try, id.id);
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    fn visit_let(&mut self, tree: &NodeTree, id: NodeId<Let>, let_node: &Let) {
        self.visit_any(tree, NodeType::Let, id.id);
    }

    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, parameter: &Parameter) {
        self.visit_any(tree, NodeType::Parameter, id.id);
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, argument: &Argument) {
        self.visit_any(tree, NodeType::Argument, id.id);
    }

    // ------------------------------------------------------------
    // Literals
    // ------------------------------------------------------------

    fn visit_scalar_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<ScalarLiteral>,
        scalar_literal: &ScalarLiteral,
    ) {
        self.visit_any(tree, NodeType::ScalarLiteral, id.id);
    }

    fn visit_range_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<RangeLiteral>,
        range_literal: &RangeLiteral,
    ) {
        self.visit_any(tree, NodeType::RangeLiteral, id.id);
    }

    fn visit_array_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<ArrayLiteral>,
        array_literal: &ArrayLiteral,
    ) {
        self.visit_any(tree, NodeType::ArrayLiteral, id.id);
    }

    fn visit_tuple_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<TupleLiteral>,
        tuple_literal: &TupleLiteral,
    ) {
        self.visit_any(tree, NodeType::TupleLiteral, id.id);
    }

    fn visit_struct_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<StructLiteral>,
        struct_literal: &StructLiteral,
    ) {
        self.visit_any(tree, NodeType::StructLiteral, id.id);
    }

    fn visit_field_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<FieldLiteral>,
        field_literal: &FieldLiteral,
    ) {
        self.visit_any(tree, NodeType::FieldLiteral, id.id);
    }

    // ------------------------------------------------------------
    // Calls
    // ------------------------------------------------------------

    fn visit_index(&mut self, tree: &NodeTree, id: NodeId<Index>, index: &Index) {
        self.visit_any(tree, NodeType::Index, id.id);
    }

    fn visit_call(&mut self, tree: &NodeTree, id: NodeId<Call>, call: &Call) {
        self.visit_any(tree, NodeType::Call, id.id);
    }

    fn visit_cast(&mut self, tree: &NodeTree, id: NodeId<Cast>, cast: &Cast) {
        self.visit_any(tree, NodeType::Cast, id.id);
    }

    fn visit_coalesce(&mut self, tree: &NodeTree, id: NodeId<Coalesce>, coalesce: &Coalesce) {
        self.visit_any(tree, NodeType::Coalesce, id.id);
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    fn visit_match(&mut self, tree: &NodeTree, id: NodeId<Match>, match_node: &Match) {
        self.visit_any(tree, NodeType::Match, id.id);
    }

    fn visit_pattern(&mut self, tree: &NodeTree, id: NodeId<Pattern>, pattern: &Pattern) {
        self.visit_any(tree, NodeType::Pattern, id.id);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        self.visit_any(tree, NodeType::PatternField, id.id);
    }

    fn visit_match_case(&mut self, tree: &NodeTree, id: NodeId<MatchCase>, match_case: &MatchCase) {
        self.visit_any(tree, NodeType::MatchCase, id.id);
    }

    // ------------------------------------------------------------
    // Annotations
    // ------------------------------------------------------------

    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Annotation>,
        annotation: &Annotation,
    ) {
        self.visit_any(tree, NodeType::Annotation, id.id);
    }

    fn visit_blank(&mut self, tree: &NodeTree, id: NodeId<Blank>, blank: &Blank) {
        self.visit_any(tree, NodeType::Blank, id.id);
    }

    fn visit_doc(&mut self, tree: &NodeTree, id: NodeId<Doc>, doc: &Doc) {
        self.visit_any(tree, NodeType::Doc, id.id);
    }

    fn visit_comment(&mut self, tree: &NodeTree, id: NodeId<Comment>, comment: &Comment) {
        self.visit_any(tree, NodeType::Comment, id.id);
    }
}

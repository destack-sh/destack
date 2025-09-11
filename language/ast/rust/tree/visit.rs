#![allow(unused_variables)]

use crate::{
    Argument, ArrayLiteral, Block, Break, Call, Cast, Coalesce, Continue, Defer, Enum, EnumField,
    Expression, FieldLiteral, For, Function, If, Implement, Index, Let, Loop, Match, MatchCase,
    Module, NodeTree, Parameter, Pattern, PatternField, RangeLiteral, Return, ScalarLiteral,
    Statement, Struct, StructField, StructLiteral, Trait, Try, Tuple, TupleField, TupleLiteral,
    Type, Union, UnionField, Use, UseClause, UseItem, While, With, WithClause, walk_argument,
    walk_array_literal, walk_block, walk_break, walk_call, walk_cast, walk_coalesce, walk_continue,
    walk_defer, walk_enum, walk_enum_field, walk_expression, walk_for, walk_function, walk_if,
    walk_implement, walk_index, walk_let, walk_loop, walk_match, walk_match_case, walk_module,
    walk_parameter, walk_pattern, walk_pattern_field, walk_range_literal, walk_return,
    walk_statement, walk_struct, walk_struct_field, walk_struct_literal, walk_trait, walk_try,
    walk_tuple, walk_tuple_field, walk_tuple_literal, walk_type, walk_union, walk_union_field,
    walk_use, walk_use_clause, walk_use_item, walk_while, walk_with, walk_with_clause,
};

/// A NodeVisitor is a visitor for the AST.
pub trait NodeVisitor {
    // ------------------------------------------------------------
    // Groupings
    // ------------------------------------------------------------

    /// Visit a Block.
    fn visit_block(&mut self, tree: &NodeTree, block: &Block) {
        walk_block(self, tree, block);
    }

    /// Visit a Statement.
    fn visit_statement(&mut self, tree: &NodeTree, statement: &Statement) {
        walk_statement(self, tree, statement);
    }

    /// Visit an Expression.
    fn visit_expression(&mut self, tree: &NodeTree, expression: &Expression) {
        walk_expression(self, tree, expression);
    }

    // ------------------------------------------------------------
    // Declarations
    // ------------------------------------------------------------

    /// Visit a Module.
    fn visit_module(&mut self, tree: &NodeTree, module: &Module) {
        walk_module(self, tree, module);
    }

    /// Visit a Struct.
    fn visit_struct(&mut self, tree: &NodeTree, struct_node: &Struct) {
        walk_struct(self, tree, struct_node);
    }

    /// Visit a StructField.
    fn visit_struct_field(&mut self, tree: &NodeTree, struct_field: &StructField) {
        walk_struct_field(self, tree, struct_field);
    }

    /// Visit an Enum.
    fn visit_enum(&mut self, tree: &NodeTree, enum_node: &Enum) {
        walk_enum(self, tree, enum_node);
    }

    /// Visit an EnumField.
    fn visit_enum_field(&mut self, tree: &NodeTree, enum_field: &EnumField) {
        walk_enum_field(self, tree, enum_field);
    }

    /// Visit a Union.
    fn visit_union(&mut self, tree: &NodeTree, union_node: &Union) {
        walk_union(self, tree, union_node);
    }

    /// Visit a UnionField.
    fn visit_union_field(&mut self, tree: &NodeTree, union_field: &UnionField) {
        walk_union_field(self, tree, union_field);
    }

    /// Visit a Trait.
    fn visit_trait(&mut self, tree: &NodeTree, trait_node: &Trait) {
        walk_trait(self, tree, trait_node);
    }

    /// Visit an Implement.
    fn visit_implement(&mut self, tree: &NodeTree, implement: &Implement) {
        walk_implement(self, tree, implement);
    }

    /// Visit a Type.
    fn visit_type(&mut self, tree: &NodeTree, type_node: &Type) {
        walk_type(self, tree, type_node);
    }

    /// Visit a Tuple.
    fn visit_tuple(&mut self, tree: &NodeTree, tuple: &Tuple) {
        walk_tuple(self, tree, tuple);
    }

    /// Visit a TupleField.
    fn visit_tuple_field(&mut self, tree: &NodeTree, tuple_field: &TupleField) {
        walk_tuple_field(self, tree, tuple_field);
    }

    /// Visit a Function.
    fn visit_function(&mut self, tree: &NodeTree, function: &Function) {
        walk_function(self, tree, function);
    }

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------

    /// Visit a With.
    fn visit_with(&mut self, tree: &NodeTree, with: &With) {
        walk_with(self, tree, with);
    }

    /// Visit a WithClause.
    fn visit_with_clause(&mut self, tree: &NodeTree, with_clause: &WithClause) {
        walk_with_clause(self, tree, with_clause);
    }

    /// Visit a Use.
    fn visit_use(&mut self, tree: &NodeTree, use_node: &Use) {
        walk_use(self, tree, use_node);
    }

    /// Visit a UseClause.
    fn visit_use_clause(&mut self, tree: &NodeTree, use_clause: &UseClause) {
        walk_use_clause(self, tree, use_clause);
    }

    /// Visit a UseItem.
    fn visit_use_item(&mut self, tree: &NodeTree, use_item: &UseItem) {
        walk_use_item(self, tree, use_item);
    }

    // ------------------------------------------------------------
    // Control
    // ------------------------------------------------------------

    /// Visit an If.
    fn visit_if(&mut self, tree: &NodeTree, if_node: &If) {
        walk_if(self, tree, if_node);
    }

    /// Visit a While.
    fn visit_while(&mut self, tree: &NodeTree, while_node: &While) {
        walk_while(self, tree, while_node);
    }

    /// Visit a For.
    fn visit_for(&mut self, tree: &NodeTree, for_node: &For) {
        walk_for(self, tree, for_node);
    }

    /// Visit a Loop.
    fn visit_loop(&mut self, tree: &NodeTree, loop_node: &Loop) {
        walk_loop(self, tree, loop_node);
    }

    /// Visit a Break.
    fn visit_break(&mut self, tree: &NodeTree, break_node: &Break) {
        walk_break(self, tree, break_node);
    }

    /// Visit a Continue.
    fn visit_continue(&mut self, tree: &NodeTree, continue_node: &Continue) {
        walk_continue(self, tree, continue_node);
    }

    /// Visit a Defer.
    fn visit_defer(&mut self, tree: &NodeTree, defer: &Defer) {
        walk_defer(self, tree, defer);
    }

    /// Visit a Return.
    fn visit_return(&mut self, tree: &NodeTree, return_node: &Return) {
        walk_return(self, tree, return_node);
    }

    /// Visit a Try.
    fn visit_try(&mut self, tree: &NodeTree, try_node: &Try) {
        walk_try(self, tree, try_node);
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    /// Visit a Let.
    fn visit_let(&mut self, tree: &NodeTree, let_node: &Let) {
        walk_let(self, tree, let_node);
    }

    /// Visit a Parameter.
    fn visit_parameter(&mut self, tree: &NodeTree, parameter: &Parameter) {
        walk_parameter(self, tree, parameter);
    }

    /// Visit an Argument.
    fn visit_argument(&mut self, tree: &NodeTree, argument: &Argument) {
        walk_argument(self, tree, argument);
    }

    // ------------------------------------------------------------
    // Literals
    // ------------------------------------------------------------

    /// Visit a ScalarLiteral.
    fn visit_scalar_literal(&mut self, tree: &NodeTree, scalar_literal: &ScalarLiteral) {}

    /// Visit a RangeLiteral.
    fn visit_range_literal(&mut self, tree: &NodeTree, range_literal: &RangeLiteral) {
        walk_range_literal(self, tree, range_literal);
    }

    /// Visit an ArrayLiteral.
    fn visit_array_literal(&mut self, tree: &NodeTree, array_literal: &ArrayLiteral) {
        walk_array_literal(self, tree, array_literal);
    }

    /// Visit a TupleLiteral.
    fn visit_tuple_literal(&mut self, tree: &NodeTree, tuple_literal: &TupleLiteral) {
        walk_tuple_literal(self, tree, tuple_literal);
    }

    /// Visit a StructLiteral.
    fn visit_struct_literal(&mut self, tree: &NodeTree, struct_literal: &StructLiteral) {
        walk_struct_literal(self, tree, struct_literal);
    }

    /// Visit a FieldLiteral.
    fn visit_field_literal(&mut self, tree: &NodeTree, field_literal: &FieldLiteral) {}

    // ------------------------------------------------------------
    // Calls
    // ------------------------------------------------------------

    /// Visit an Index.
    fn visit_index(&mut self, tree: &NodeTree, index: &Index) {
        walk_index(self, tree, index);
    }

    /// Visit a Call.
    fn visit_call(&mut self, tree: &NodeTree, call: &Call) {
        walk_call(self, tree, call);
    }

    /// Visit a Cast.
    fn visit_cast(&mut self, tree: &NodeTree, cast: &Cast) {
        walk_cast(self, tree, cast);
    }

    /// Visit a Coalesce.
    fn visit_coalesce(&mut self, tree: &NodeTree, coalesce: &Coalesce) {
        walk_coalesce(self, tree, coalesce);
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    /// Visit a Match.
    fn visit_match(&mut self, tree: &NodeTree, match_node: &Match) {
        walk_match(self, tree, match_node);
    }

    /// Visit a MatchCase.
    fn visit_match_case(&mut self, tree: &NodeTree, match_case: &MatchCase) {
        walk_match_case(self, tree, match_case);
    }

    /// Visit a Pattern.
    fn visit_pattern(&mut self, tree: &NodeTree, pattern: &Pattern) {
        walk_pattern(self, tree, pattern);
    }

    /// Visit a PatternField.
    fn visit_pattern_field(&mut self, tree: &NodeTree, pattern_field: &PatternField) {
        walk_pattern_field(self, tree, pattern_field);
    }
}

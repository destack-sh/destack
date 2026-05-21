use destack_dir as dir;

use crate::DiagnosticAnchor;

use super::TypeInferId;

/// Rule validated after solving.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Obligation {
    /// Source must be assignable to target.
    ///
    /// ```ds
    /// let value: int32 = source;
    /// // type(source) must be assignable to int32
    /// ```
    Assignable {
        /// The source type.
        source: TypeInferId,
        /// The target type.
        target: TypeInferId,
        /// The diagnostic context.
        context: ObligationContext,
    },
    /// Value must satisfy a constraint.
    ///
    /// ```ds
    /// function sort<T: Comparable>(values: T[]) {}
    /// // T must satisfy Comparable
    /// ```
    Satisfies {
        /// The value type.
        value: TypeInferId,
        /// The constraint type.
        constraint: TypeInferId,
        /// The diagnostic context.
        context: ObligationContext,
    },
    /// Subtype must extend supertype.
    ///
    /// ```ds
    /// class Child extends Parent {}
    /// // Child must extend Parent
    /// ```
    Extends {
        /// The subtype.
        subtype: TypeInferId,
        /// The supertype.
        supertype: TypeInferId,
        /// The diagnostic context.
        context: ObligationContext,
    },
    /// Implementor must implement contract.
    ///
    /// ```ds
    /// class User implements Serializable {}
    /// // User must implement Serializable
    /// ```
    Implements {
        /// The implementing type.
        implementor: TypeInferId,
        /// The contract type.
        contract: TypeInferId,
        /// The diagnostic context.
        context: ObligationContext,
    },
    /// Type must have a concrete layout.
    ///
    /// ```ds
    /// sizeOf<T>()
    /// // T must have a concrete layout
    /// ```
    ConcreteLayout {
        /// The type to lay out.
        ty: TypeInferId,
        /// The diagnostic context.
        context: ObligationContext,
    },
    /// Call expression must resolve to a callable target.
    ///
    /// ```ds
    /// parse(text);
    /// // parse must be callable with text
    /// ```
    Callable {
        /// The call expression.
        call: dir::GlobalNodeIdAny,
        /// The callee type.
        callee: TypeInferId,
        /// The argument types.
        arguments: Vec<TypeInferId>,
        /// The diagnostic context.
        context: ObligationContext,
    },
}

/// Diagnostic context for an obligation.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ObligationContext {
    /// The diagnostic anchor.
    pub(in crate::check) anchor: DiagnosticAnchor,
    /// The obligation source context.
    pub(in crate::check) source: ObligationSource,
}

/// Source of an obligation.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum ObligationSource {
    /// Declaration type annotation.
    ///
    /// ```ds
    /// let value: int32;
    /// // report against the declared type
    /// ```
    DeclarationType {
        /// The declaration node.
        declaration: dir::GlobalNodeIdAny,
    },
    /// Variable initializer assignment.
    ///
    /// ```ds
    /// let value: int32 = source;
    /// // report against the initializer of value
    /// ```
    VariableInitializer {
        /// The variable symbol.
        symbol: dir::GlobalSymbolId,
    },
    /// Function return assignment.
    ///
    /// ```ds
    /// function parse(): int32 {
    ///   return text;
    ///   // report against this return value
    /// }
    /// ```
    FunctionReturn {
        /// The function symbol.
        function: dir::GlobalSymbolId,
        /// The returned node.
        return_node: dir::GlobalNodeIdAny,
    },
    /// Call argument assignment.
    ///
    /// ```ds
    /// parse(text);
    /// // report against the argument that failed its parameter type
    /// ```
    CallArgument {
        /// The call node.
        call: dir::GlobalNodeIdAny,
        /// The parameter symbol, when known.
        parameter: Option<dir::GlobalSymbolId>,
        /// The argument index.
        index: usize,
    },
    /// Generic or static argument bound check.
    ///
    /// ```ds
    /// sort<NotComparable>(values);
    /// // report against NotComparable
    /// ```
    GenericArgument {
        /// The generic application node.
        application: dir::GlobalNodeIdAny,
        /// The parameter symbol, when known.
        parameter: Option<dir::GlobalSymbolId>,
        /// The argument index.
        index: usize,
    },
    /// Static argument.
    ///
    /// ```ds
    /// Array<T, length()>
    /// // report against length()
    /// ```
    StaticArgument {
        /// The static application node.
        application: dir::GlobalNodeIdAny,
        /// The parameter symbol, when known.
        parameter: Option<dir::GlobalSymbolId>,
        /// The argument index.
        index: usize,
    },
    /// Call expression selection.
    ///
    /// ```ds
    /// parse(text);
    /// // report against the call expression
    /// ```
    Call {
        /// The call node.
        call: dir::GlobalNodeIdAny,
    },
    /// Extends clause.
    ///
    /// ```ds
    /// class Child extends Parent {}
    /// // report against Parent
    /// ```
    ExtendsClause {
        /// The extending declaration.
        symbol: dir::GlobalSymbolId,
        /// The extends clause node.
        clause: dir::GlobalNodeIdAny,
    },
    /// Implements clause.
    ///
    /// ```ds
    /// class User implements Serializable {}
    /// // report against Serializable
    /// ```
    ImplementsClause {
        /// The implementing declaration.
        symbol: dir::GlobalSymbolId,
        /// The implements clause node.
        clause: dir::GlobalNodeIdAny,
    },
    /// Exported type must be known.
    ///
    /// ```ds
    /// export const value = compute();
    /// // report when the exported type cannot be solved
    /// ```
    ExportedDeclaration {
        /// The exported symbol.
        symbol: dir::GlobalSymbolId,
    },
    /// Static condition.
    ///
    /// ```ds
    /// @if(flag()) {}
    /// // report when flag cannot be evaluated statically
    /// ```
    StaticCondition {
        /// The static condition node.
        condition: dir::GlobalNodeIdAny,
    },
    /// Explicit layout request.
    ///
    /// ```ds
    /// sizeOf<T>();
    /// // report when T has no concrete representation
    /// ```
    LayoutRequest {
        /// The layout request node.
        request: dir::GlobalNodeIdAny,
    },
}

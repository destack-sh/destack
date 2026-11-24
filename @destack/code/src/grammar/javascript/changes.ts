/** A change of a rule of Lezer's grammar of JavaScript. */
export interface GrammarChange {
    /** The rule as it is published. */
    readonly from: string;
    /** The rule as it is here. */
    readonly to: string;
}

/** The forms of TypeScript and JavaScript the published grammar lacks. */
export const JAVASCRIPT_CHANGES: readonly GrammarChange[] = [
    // (row): row is Row => …
    {
        from: `async? (ParamList { VariableDefinition } | ParamList TypeAnnotation?) "=>" (Block | expressionNoComma)`,
        to: `async? (ParamList { VariableDefinition } | ParamList (TypeAnnotation | TypePredicate)?) "=>" (Block | expressionNoComma)`,
    },

    // export { type A }, export type * from, and attributes
    {
        from: `"{" commaSep<(VariableName | String | kw<"default">) (ckw<"as"> (VariableName { word } | String))?> "}"`,
        to: `"{" commaSep<tskw<"type">? (VariableName | String | kw<"default">) (ckw<"as"> (VariableName { word } | String))?> "}"`,
    },
    {
        from: `kw<"export"> Star (ckw<"as"> (VariableName | String))? ckw<"from"> String semi |`,
        to: `kw<"export"> tskw<"type">? Star (ckw<"as"> (VariableName | String))? ckw<"from"> String ImportAttributes? semi |`,
    },
    {
        from: `kw<"export"> tskw<"type">? ExportGroup (ckw<"from"> String)? semi |`,
        to: `kw<"export"> tskw<"type">? ExportGroup (ckw<"from"> String ImportAttributes?)? semi |`,
    },
    {
        from: `    ckw<"from"> String semi |
  kw<"import"> ckw<"defer">? String semi
}`,
        to: `    ckw<"from"> String ImportAttributes? semi |
  kw<"import"> ckw<"defer">? String ImportAttributes? semi
}

ImportAttributes { kw<"with"> "{" commaSep<(PropertyName | String) ":" String> "}" }`,
    },
    {
        from: `DynamicImport { kw<"import"> "(" expressionNoComma ")" }`,
        to: `DynamicImport { kw<"import"> "(" expressionNoComma ("," expressionNoComma?)? ")" }`,
    },

    // declare module "x"
    {
        from: `(tskw<"namespace"> | tskw<"module">) VariableDefinition ("." PropertyDefinition)* Block`,
        to: `(tskw<"namespace"> | tskw<"module">) (VariableDefinition ("." PropertyDefinition)* | String) Block`,
    },

    // class implements A {}, abstract class {}
    {
        from: `kw<"class"> VariableDefinition? (kw<"extends"> expression)? ClassBody`,
        to: `tskw<"abstract">? kw<"class"> VariableDefinition? TypeParamList?
  (kw<"extends"> ((VariableName | MemberExpression) !typeargs TypeArgList | expression))?
  (tskw<"implements"> commaSep1<type>)?
  ClassBody`,
    },

    // types: infer A extends B, [label?: A, B?], in out T, typeof import("x"), abstract new () => A
    {
        from: `InferredType { tskw<"infer"> TypeName } |`,
        to: `InferredType { tskw<"infer"> TypeName (!inferExtends kw<"extends"> type)? } |`,
    },
    { from: `  typeExtends @right,`, to: `  inferExtends,\n  typeExtends @right,` },
    {
        from: `TupleType { "[" commaSep<(Label ":")? type | "..." type> ~destructure "]" } |`,
        to: `TupleType { "[" commaSep<(Label Optional? ":")? type Optional? | "..." (Label ":")? type> ~destructure "]" } |`,
    },
    { from: `Label { identifier }`, to: `Label { identifier ~arrow }` },
    {
        from: `typeParam { (kw<"in"> | tskw<"out"> | kw<"const">)? TypeDefinition`,
        to: `typeParam { (kw<"in"> | tskw<"out"> | kw<"const">)* TypeDefinition`,
    },
    {
        from: `TypeofType { kw<"typeof"> typeofExpression } |`,
        to: `TypeofType { kw<"typeof"> (typeofExpression | ImportType { kw<"import"> "(" String ")" }) } |`,
    },
    {
        from: `NewSignature { kw<"new"> ParamTypeList "=>" type } |`,
        to: `NewSignature { tskw<"abstract">? kw<"new"> TypeParamList? ParamTypeList "=>" type } |`,
    },

    // (row: Row) => row is Plan
    {
        from: `FunctionSignature { TypeParamList? ParamTypeList "=>" type } |`,
        to: `FunctionSignature { TypeParamList? ParamTypeList "=>" (type | PredicateType) } |`,
    },
    {
        from: `IndexedType {
  type !typeMember`,
        to: `PredicateType[@name=TypePredicate] {
  tskw<"asserts"> (VariableName | kw<"this">) !predicate (tskw<"is"> type)? |
  (VariableName | kw<"this">) !predicate tskw<"is"> type
}

IndexedType {
  type !typeMember`,
    },

    // { <T>(value: T): T }, new <T>(): T, and an index signature of a class
    {
        from: `CallSignature { ParamTypeList (TypeAnnotation | TypePredicate) } |`,
        to: `CallSignature { TypeParamList? ParamTypeList (TypeAnnotation | TypePredicate) } |`,
    },
    {
        from: `classItem { MethodDeclaration | PropertyDeclaration | StaticBlock | ";" }`,
        to: `classItem { MethodDeclaration | PropertyDeclaration | StaticBlock | IndexSignature semi | ";" }`,
    },

    // catch (error: unknown), {/* a comment */}, and [this.a, b[0]] = c
    {
        from: `CatchClause { kw<"catch"> ("(" pattern ")")? Block }?`,
        to: `CatchClause { kw<"catch"> ("(" pattern TypeAnnotation? ")")? Block }?`,
    },
    {
        from: `JSXEscape { "{" "..."? expression "}" }`,
        to: `JSXEscape { "{" ("..."? expression)? "}" }`,
    },
    {
        from: `(VariableName | MemberExpression | ArrayPattern | ObjectPattern) !assign "=" expressionNoComma`,
        to: `(VariableName | MemberExpression | ArrayPattern | ObjectPattern | ArrayTarget | ObjectTarget) !assign "=" expressionNoComma`,
    },
    {
        from: `patternAssign {
  pattern`,
        to: `ArrayTarget[@name=ArrayPattern,@dynamicPrecedence=-1] { ArrayExpression }

ObjectTarget[@name=ObjectPattern,@dynamicPrecedence=-1] { ObjectExpression }

patternAssign {
  pattern`,
    },
    {
        from: `  ObjectExpression { "{" commaSep<Property> ~destructure "}" } |`,
        to: `  ObjectExpression |`,
    },
    {
        from: `ArrayExpression {
  "["`,
        to: `ObjectExpression { "{" commaSep<Property> ~destructure "}" }

ArrayExpression {
  "["`,
    },
];

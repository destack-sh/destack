import { styleTags, tags } from "@lezer/highlight";

/** What each node of the grammar is. */
export const jsHighlight = styleTags({
    // keywords
    "get set async static": tags.modifier,
    "for while do if else switch try catch finally return throw break continue default case defer":
        tags.controlKeyword,
    "in of await yield void typeof delete instanceof as satisfies": tags.operatorKeyword,
    "let var const using function class extends": tags.definitionKeyword,
    "import export from": tags.moduleKeyword,
    "with debugger new": tags.keyword,
    "type enum interface implements namespace module declare": tags.definitionKeyword,
    "abstract global Privacy readonly override": tags.modifier,
    "is keyof unique infer asserts out": tags.operatorKeyword,

    // values
    TemplateString: tags.special(tags.string),
    String: tags.string,
    Escape: tags.escape,
    RegExp: tags.regexp,
    Number: tags.number,
    BooleanLiteral: tags.bool,
    null: tags.null,
    this: tags.self,
    super: tags.atom,

    // names
    VariableName: tags.variableName,
    VariableDefinition: tags.definition(tags.variableName),
    Label: tags.labelName,
    PropertyName: tags.propertyName,
    PropertyDefinition: tags.definition(tags.propertyName),
    PrivatePropertyName: tags.special(tags.propertyName),
    PrivatePropertyDefinition: tags.definition(tags.special(tags.propertyName)),

    // functions
    "CallExpression/VariableName TaggedTemplateExpression/VariableName": tags.function(
        tags.variableName,
    ),
    "CallExpression/MemberExpression/PropertyName": tags.function(tags.propertyName),
    "CallExpression/MemberExpression/PrivatePropertyName": tags.function(tags.propertyName),
    "FunctionDeclaration/VariableDefinition AmbientFunctionDeclaration/VariableDefinition":
        tags.function(tags.definition(tags.variableName)),
    "FunctionExpression/VariableDefinition": tags.function(tags.definition(tags.variableName)),
    "MethodDeclaration/PropertyDefinition MethodType/PropertyDefinition": tags.function(
        tags.definition(tags.propertyName),
    ),
    "MethodDeclaration/PrivatePropertyDefinition": tags.function(
        tags.definition(tags.propertyName),
    ),

    // types
    TypeName: tags.typeName,
    TypeDefinition: tags.definition(tags.typeName),
    "ClassDeclaration/VariableDefinition ClassExpression/VariableDefinition": tags.definition(
        tags.className,
    ),
    "ClassDeclaration/VariableName ClassExpression/VariableName": tags.className,
    "NewExpression/VariableName": tags.className,
    "VoidType NullType ThisType": tags.typeName,

    // elements
    JSXAttributeValue: tags.attributeValue,
    JSXText: tags.content,
    "JSXStartTag JSXStartCloseTag JSXSelfCloseEndTag JSXEndTag": tags.angleBracket,
    "JSXIdentifier JSXNamespacedName": tags.tagName,
    "JSXAttribute/JSXIdentifier JSXAttribute/JSXNamespacedName": tags.attributeName,

    // marks
    "LineComment Hashbang": tags.lineComment,
    BlockComment: tags.blockComment,
    UpdateOp: tags.updateOperator,
    ArithOp: tags.arithmeticOperator,
    LogicOp: tags.logicOperator,
    BitOp: tags.bitwiseOperator,
    CompareOp: tags.compareOperator,
    Equals: tags.definitionOperator,
    Star: tags.modifier,
    Arrow: tags.punctuation,
    ": Spread": tags.punctuation,
    "( )": tags.paren,
    "[ ]": tags.squareBracket,
    "{ }": tags.brace,
    "InterpolationStart InterpolationEnd": tags.special(tags.brace),
    ".": tags.derefOperator,
    ", ;": tags.separator,
    "@": tags.meta,

    // angle brackets around types
    "TypeArgList/< TypeArgList/> TypeParamList/< TypeParamList/>": tags.angleBracket,
});

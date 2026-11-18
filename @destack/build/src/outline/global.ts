import type { ESTree } from "rolldown/utils";
import type { GlobalReference } from "@destack/package/code";
import { children } from "./node.ts";

/** The nodes whose bodies run later than the module, when something calls them. */
const FUNCTION_KINDS = new Set([
    "FunctionDeclaration",
    "FunctionExpression",
    "ArrowFunctionExpression",
]);

/** The TypeScript nodes that hold runtime expressions; every other TypeScript node is a type. */
const RUNTIME_TYPESCRIPT_KINDS = new Set([
    "TSAsExpression",
    "TSSatisfiesExpression",
    "TSNonNullExpression",
    "TSTypeAssertion",
    "TSInstantiationExpression",
    "TSEnumDeclaration",
    "TSEnumBody",
    "TSEnumMember",
    "TSModuleDeclaration",
    "TSModuleBlock",
    "TSParameterProperty",
    "TSExportAssignment",
    "TSImportEqualsDeclaration",
    "TSExternalModuleReference",
]);

/** The statements that bind other modules' names, which hold no runtime expression. */
const MODULE_KINDS = new Set(["ImportDeclaration", "ExportAllDeclaration"]);

/** The nodes that keep their function-scoped declarations: functions, classes and namespaces. */
const FUNCTION_SCOPE_KINDS = new Set([
    ...FUNCTION_KINDS,
    "ClassDeclaration",
    "ClassExpression",
    "StaticBlock",
    "TSModuleDeclaration",
]);

/** The names the language provides as values rather than as runtime globals. */
const INTRINSICS = new Set(["undefined"]);

/** A scope: the names declared in it. */
type Scope = Set<string>;

/** Collect the globals a module uses: the names its expressions read that nothing declares. */
export function collectGlobals(program: ESTree.Program, file: string): GlobalReference[] {
    const globals: GlobalReference[] = [];
    walk(program, [], [], (node, scopes, ancestors) => {
        // take an identifier read as a value that no enclosing scope declares
        if (node.type !== "Identifier" || !isRead(node, ancestors)) {
            return;
        }
        const isDeclared = scopes.some((scope) => scope.has(node.name));
        if (isDeclared || INTRINSICS.has(node.name) || isArguments(node, ancestors)) {
            return;
        }

        // record it with the members its access reads and when it runs
        const { members, isDynamic, end } = accessPath(node, ancestors);
        globals.push({
            name: node.name,
            members,
            dynamic: isDynamic,
            evaluation: evaluation(ancestors),
            source: { file, start: node.start, end },
        });
    });

    return globals;
}

/** Walk a node's runtime expressions with the scopes and ancestors around each, skipping types. */
function walk(
    node: ESTree.Node,
    scopes: Scope[],
    ancestors: ESTree.Node[],
    visit: (node: ESTree.Node, scopes: readonly Scope[], ancestors: readonly ESTree.Node[]) => void,
): void {
    // skip types, imports, and declared ambient modules
    const isType = node.type.startsWith("TS") && !RUNTIME_TYPESCRIPT_KINDS.has(node.type);
    const isAmbient = node.type === "TSModuleDeclaration" && node.declare;
    if (isType || isAmbient || MODULE_KINDS.has(node.type)) {
        return;
    }
    visit(node, scopes, ancestors);

    // walk its children inside the scope it opens
    const scope = opened(node);
    if (scope !== undefined) {
        scopes.push(scope);
    }
    ancestors.push(node);
    for (const child of children(node)) {
        walk(child, scopes, ancestors, visit);
    }
    ancestors.pop();
    if (scope !== undefined) {
        scopes.pop();
    }
}

/** Read the scope a node opens, absent for a node that opens none. */
function opened(node: ESTree.Node): Scope | undefined {
    // open the module's, a function's or a namespace's declarations with the vars they hoist
    if (node.type === "Program" || node.type === "TSModuleBlock" || node.type === "StaticBlock") {
        return new Set([...lexical(node.body), ...hoistedNames(node.body)]);
    } else if (FUNCTION_KINDS.has(node.type) && "params" in node) {
        const scope = new Set<string>();
        if (node.type === "FunctionExpression" && node.id !== null) {
            scope.add(node.id.name);
        }
        for (const parameter of node.params) {
            bindings(parameter, scope);
        }
        const body = "body" in node ? node.body : null;
        const statements = body?.type === "BlockStatement" ? body.body : [];

        return new Set([...scope, ...lexical(statements), ...hoistedNames(statements)]);
    }
    // open a block's, a switch's and a loop's own declarations, a catch parameter, and a class's name
    else if (node.type === "BlockStatement") {
        return lexical(node.body);
    } else if (node.type === "SwitchStatement") {
        return lexical(node.cases.flatMap((each) => each.consequent));
    } else if (
        node.type === "ForStatement" ||
        node.type === "ForInStatement" ||
        node.type === "ForOfStatement"
    ) {
        const declaration = node.type === "ForStatement" ? node.init : node.left;
        const scope = new Set<string>();
        if (declaration?.type === "VariableDeclaration") {
            for (const declarator of declaration.declarations) {
                bindings(declarator.id, scope);
            }
        }

        return scope;
    } else if (node.type === "CatchClause") {
        const scope = new Set<string>();
        bindings(node.param, scope);

        return scope;
    } else if (node.type === "ClassExpression" && node.id !== null) {
        return new Set([node.id.name]);
    }

    return undefined;
}

/** List the runtime names statements declare in their own block, not hoisted names or types. */
function lexical(statements: readonly (ESTree.Directive | ESTree.Statement)[]): Scope {
    const scope = new Set<string>();
    for (const statement of statements) {
        const declaration =
            statement.type === "ExportNamedDeclaration" ||
            statement.type === "ExportDefaultDeclaration"
                ? statement.declaration
                : statement;

        // name an import's locals, a block-scoped variable's bindings, and a value declaration's own name
        if (declaration === null) {
            continue;
        } else if (declaration.type === "ImportDeclaration") {
            for (const specifier of declaration.specifiers) {
                scope.add(specifier.local.name);
            }
        } else if (declaration.type === "VariableDeclaration") {
            for (const declarator of declaration.kind === "var" ? [] : declaration.declarations) {
                bindings(declarator.id, scope);
            }
        } else if (
            declaration.type !== "TSInterfaceDeclaration" &&
            declaration.type !== "TSTypeAliasDeclaration" &&
            !(declaration.type === "TSModuleDeclaration" && declaration.global) &&
            "id" in declaration &&
            declaration.id?.type === "Identifier"
        ) {
            scope.add(declaration.id.name);
        }
    }

    return scope;
}

/** List the function-scoped names statements declare, through nested blocks but not functions. */
function hoistedNames(statements: readonly (ESTree.Directive | ESTree.Statement)[]): Scope {
    // visit the statements and their nested blocks, leaving functions, classes and namespaces
    const scope = new Set<string>();
    const pending: ESTree.Node[] = [...statements];
    for (let node = pending.pop(); node !== undefined; node = pending.pop()) {
        if (node.type === "VariableDeclaration" && node.kind === "var") {
            for (const declarator of node.declarations) {
                bindings(declarator.id, scope);
            }
        } else if (!FUNCTION_SCOPE_KINDS.has(node.type)) {
            pending.push(...children(node));
        }
    }

    return scope;
}

/** Add the names a binding pattern binds to a scope. */
function bindings(pattern: ESTree.Node | null, scope: Scope): void {
    // name an identifier
    if (pattern === null) {
        return;
    } else if (pattern.type === "Identifier") {
        scope.add(pattern.name);
    }
    // name each property, element, default, rest and parameter property of a pattern
    else if (pattern.type === "ObjectPattern") {
        for (const property of pattern.properties) {
            bindings(property.type === "RestElement" ? property : property.value, scope);
        }
    } else if (pattern.type === "ArrayPattern") {
        for (const element of pattern.elements) {
            bindings(element, scope);
        }
    } else if (pattern.type === "AssignmentPattern") {
        bindings(pattern.left, scope);
    } else if (pattern.type === "RestElement") {
        bindings(pattern.argument, scope);
    } else if (pattern.type === "TSParameterProperty") {
        bindings(pattern.parameter, scope);
    }
}

/** Report whether an identifier is read as a value, not a name, key, pattern or label. */
function isRead(node: ESTree.Node, ancestors: readonly ESTree.Node[]): boolean {
    // read an identifier outside any construct
    const parent = ancestors.at(-1);
    const grandparent = ancestors.at(-2);
    if (parent === undefined) {
        return true;
    }
    // read a member access's object and computed key, a property's value and computed key
    else if (parent.type === "MemberExpression") {
        return parent.object === node || parent.computed;
    } else if (parent.type === "Property") {
        const isPattern = grandparent?.type === "ObjectPattern";

        return parent.computed ? parent.key === node : parent.value === node && !isPattern;
    } else if (
        parent.type === "PropertyDefinition" ||
        parent.type === "AccessorProperty" ||
        parent.type === "MethodDefinition"
    ) {
        return parent.computed ? parent.key === node : parent.value === node;
    }
    // read an initializer, a function's expression body, a default's value and a class's base
    else if (parent.type === "VariableDeclarator") {
        return parent.init === node;
    } else if (FUNCTION_KINDS.has(parent.type)) {
        return "body" in parent && parent.body === node;
    } else if (parent.type === "AssignmentPattern") {
        return parent.right === node;
    } else if (parent.type === "ClassDeclaration" || parent.type === "ClassExpression") {
        return parent.superClass === node;
    } else if (parent.type === "TSImportEqualsDeclaration") {
        return parent.id !== node;
    }

    // leave labels, catch parameters, patterns and the names other declarations give
    const isNaming =
        parent.type === "LabeledStatement" ||
        parent.type === "BreakStatement" ||
        parent.type === "ContinueStatement" ||
        parent.type === "CatchClause" ||
        parent.type === "ArrayPattern" ||
        parent.type === "RestElement" ||
        parent.type === "ExportSpecifier" ||
        parent.type === "TSEnumDeclaration" ||
        parent.type === "TSEnumMember" ||
        parent.type === "TSModuleDeclaration" ||
        parent.type === "TSParameterProperty" ||
        parent.type === "MetaProperty";

    return !isNaming;
}

/** Report whether an identifier is a non-arrow function's own `arguments`. */
function isArguments(
    node: ESTree.IdentifierReference | ESTree.Node,
    ancestors: readonly ESTree.Node[],
): boolean {
    return (
        "name" in node &&
        node.name === "arguments" &&
        ancestors.some(
            (ancestor) =>
                ancestor.type === "FunctionDeclaration" || ancestor.type === "FunctionExpression",
        )
    );
}

/** Read the members an access reads off an identifier, and whether a computed key hides one. */
function accessPath(
    node: ESTree.Node,
    ancestors: readonly ESTree.Node[],
): { members: string[]; isDynamic: boolean; end: number } {
    // walk outward through the accesses
    const members: string[] = [];
    let expression = node;
    for (const parent of ancestors.toReversed()) {
        // extend the path by a static name or a literal key, ending at a well-known symbol key
        if (parent.type !== "MemberExpression" || parent.object !== expression) {
            break;
        } else if (!parent.computed) {
            members.push(parent.property.name);
        } else if (
            parent.property.type === "Literal" &&
            (typeof parent.property.value === "string" || typeof parent.property.value === "number")
        ) {
            members.push(String(parent.property.value));
        } else if (isWellKnownSymbol(parent.property)) {
            break;
        } else {
            return { members, isDynamic: true, end: parent.end };
        }
        expression = parent;
    }

    return { members, isDynamic: false, end: expression.end };
}

/** Report whether a key is a well-known symbol, such as `Symbol.iterator`. */
function isWellKnownSymbol(node: ESTree.Node): boolean {
    return (
        node.type === "MemberExpression" &&
        !node.computed &&
        node.object.type === "Identifier" &&
        node.object.name === "Symbol"
    );
}

/** Tell when an expression runs: as the module evaluates, or once a function around it runs. */
function evaluation(ancestors: readonly ESTree.Node[]): GlobalReference["evaluation"] {
    // walk outward to the first function or instance field that defers the expression
    for (let index = ancestors.length - 1; index >= 0; index -= 1) {
        const ancestor = ancestors[index];
        const parent = ancestors[index - 1];
        if (ancestor === undefined) {
            continue;
        }

        // defer a function's body and parameters, unless the module calls the function right away
        const isFunction =
            FUNCTION_KINDS.has(ancestor.type) || ancestor.type === "MethodDefinition";
        const isCalledAtOnce = parent?.type === "CallExpression" && parent.callee === ancestor;
        if (isFunction && !isCalledAtOnce) {
            return "function";
        }
        // defer an instance field's initializer to construction
        else if (
            (ancestor.type === "PropertyDefinition" || ancestor.type === "AccessorProperty") &&
            !ancestor.static
        ) {
            return "function";
        }
    }

    return "module";
}

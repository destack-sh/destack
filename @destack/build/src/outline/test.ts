import type { ESTree } from "rolldown/utils";
import { schema } from "@destack/schema";
import { BuildError } from "../error/index.ts";
import { children } from "./node.ts";

/** The packages whose test and suite functions register tests. */
const TEST_PACKAGES = new Set(["@destack/test", "vitest"]);

/** The test and suite functions, by the name each package exports them under. */
const TEST_FUNCTIONS: Readonly<Record<string, "test" | "suite">> = {
    test: "test",
    it: "test",
    describe: "suite",
    suite: "suite",
};

/** The statements and expressions that register tests only on some executions. */
const CONDITIONAL_KINDS = new Set([
    "IfStatement",
    "ForStatement",
    "ForOfStatement",
    "ForInStatement",
    "WhileStatement",
    "DoWhileStatement",
    "ConditionalExpression",
    "SwitchStatement",
    "LogicalExpression",
    "BinaryExpression",
]);

/** The nodes whose bodies run later than the module, when something calls them. */
const DEFERRING_KINDS = new Set([
    "FunctionDeclaration",
    "FunctionExpression",
    "ArrowFunctionExpression",
    "MethodDefinition",
    "PropertyDefinition",
    "StaticBlock",
]);

/** The test function members that return or reconfigure a test function instead of registering. */
const FIXTURE_MODIFIERS = new Set(["extend", "override", "scoped"]);

/** A statically identified test or suite declaration. */
export const TestDeclaration = schema.object({
    /** Whether the call declares a case or a suite. */
    kind: schema.enum(["test", "suite"]),
    /** The literal title or parameterized title pattern. */
    name: schema.string(),
    /** The titles of the enclosing suites, outermost first. */
    suites: schema.array(schema.string()),
    /** The source file relative to the package root. */
    file: schema.string(),
    /** The inclusive UTF-16 source offset. */
    start: schema.number().int().min(0),
    /** The exclusive UTF-16 source offset. */
    end: schema.number().int().min(0),
    /** Chained options such as skip, only, concurrent, or each. */
    modifiers: schema.array(schema.string()),
});
/** A statically identified test or suite declaration. */
export type TestDeclaration = schema.Infer<typeof TestDeclaration>;

/** The test and suite functions a module binds: by local name, and the namespaces holding them. */
interface Runners {
    /** The kind of each function by its local name. */
    readonly functions: Map<string, "test" | "suite">;
    /** The local names of namespace imports of a test package. */
    readonly namespaces: Set<string>;
}

/** Collect a module's test and suite declarations from its syntax, without evaluating them. */
export function collectTests(program: ESTree.Program, file: string): TestDeclaration[] {
    // bind the test and suite functions and namespaces the module imports, by local name
    const runners: Runners = { functions: new Map(), namespaces: new Set() };
    for (const statement of program.body) {
        if (statement.type === "ImportDeclaration" && TEST_PACKAGES.has(statement.source.value)) {
            for (const specifier of statement.specifiers) {
                const imported =
                    specifier.type === "ImportSpecifier" && specifier.imported.type === "Identifier"
                        ? TEST_FUNCTIONS[specifier.imported.name]
                        : undefined;
                if (specifier.type === "ImportNamespaceSpecifier") {
                    runners.namespaces.add(specifier.local.name);
                } else if (imported !== undefined) {
                    runners.functions.set(specifier.local.name, imported);
                }
            }
        }
    }
    if (runners.functions.size === 0 && runners.namespaces.size === 0) {
        return [];
    }

    // bind the test functions the module's constants extend from bound ones, such as `test.extend`
    for (const statement of program.body) {
        const declaration =
            statement.type === "ExportNamedDeclaration" ? statement.declaration : statement;
        for (const { id, init } of declaration?.type === "VariableDeclaration"
            ? declaration.declarations
            : []) {
            const called = init === null ? undefined : calledRunner(init, runners);
            const isExtended = called?.modifiers.some((each) => FIXTURE_MODIFIERS.has(each));
            if (id.type === "Identifier" && called !== undefined && isExtended === true) {
                runners.functions.set(id.name, called.kind);
            }
        }
    }

    // walk the module, keeping each node's ancestors and the suites each suite callback opens
    const declarations: TestDeclaration[] = [];
    const suites = new Map<ESTree.Node, string[]>();
    const ancestors: ESTree.Node[] = [];
    const visit = (node: ESTree.Node) => {
        // describe an outermost call of a bound test or suite function
        const parent = ancestors.at(-1);
        const isOutermost =
            parent === undefined || parent.type !== "CallExpression" || parent.callee !== node;
        if (node.type === "CallExpression" && isOutermost) {
            const declared = describeCall(node, ancestors, runners, suites, file);
            if (declared !== undefined) {
                declarations.push(declared);
            }
        }

        // visit every child node with this node as its parent
        ancestors.push(node);
        for (const child of children(node)) {
            visit(child);
        }
        ancestors.pop();
    };
    visit(program);

    return declarations;
}

/** Follow a call's modifiers and parameterized calls to its bound runner, absent for any other. */
function calledRunner(
    node: ESTree.Node,
    runners: Runners,
): { readonly kind: "test" | "suite"; readonly modifiers: string[] } | undefined {
    // unwrap member accesses, calls and tagged templates down to the called name
    const modifiers: string[] = [];
    let expression: ESTree.Node = node;
    while (
        expression.type === "MemberExpression" ||
        expression.type === "CallExpression" ||
        expression.type === "TaggedTemplateExpression"
    ) {
        if (expression.type === "MemberExpression") {
            if (expression.computed) {
                return undefined;
            }
            modifiers.unshift(expression.property.name);
            expression = expression.object;
        } else if (expression.type === "CallExpression") {
            expression = expression.callee;
        } else {
            expression = expression.tag;
        }
    }

    // take a bound function, or a test function of a bound namespace
    const name = expression.type === "Identifier" ? expression.name : undefined;
    const bound = name === undefined ? undefined : runners.functions.get(name);
    const isNamespace = name !== undefined && runners.namespaces.has(name);
    const member = isNamespace ? modifiers.shift() : undefined;
    const kind = bound ?? (member === undefined ? undefined : TEST_FUNCTIONS[member]);

    return kind === undefined ? undefined : { kind, modifiers };
}

/** Describe a call registering a test or suite in module or suite scope, absent for any other. */
function describeCall(
    node: ESTree.CallExpression,
    ancestors: readonly ESTree.Node[],
    runners: Runners,
    suites: Map<ESTree.Node, string[]>,
    file: string,
): TestDeclaration | undefined {
    // take a call of a bound runner, leaving the calls that extend a runner
    const called = calledRunner(node.callee, runners);
    if (called === undefined || called.modifiers.some((each) => FIXTURE_MODIFIERS.has(each))) {
        return undefined;
    }
    const { kind, modifiers } = called;

    // leave registrations in other functions to the modules calling them
    const scopes = ancestors.toReversed();
    if (scopes.some((scope) => DEFERRING_KINDS.has(scope.type) && !suites.has(scope))) {
        return undefined;
    }

    // refuse a registration whose presence depends on executing application code
    const location = `${file}:${String(node.start)}`;
    if (scopes.some((scope) => CONDITIONAL_KINDS.has(scope.type))) {
        throw BuildError.of(
            "INSPECTION_INVALID",
            `test declaration at '${location}' is invalid: it requires module or suite scope`,
        );
    }

    // require a title read without evaluation, and the innermost suite's titles
    const [title] = node.arguments;
    const name = title === undefined ? undefined : literalText(title);
    if (name === undefined) {
        throw BuildError.of(
            "INSPECTION_INVALID",
            `test declaration at '${location}' is invalid: it requires a literal title`,
        );
    }
    const titles =
        scopes.map((scope) => suites.get(scope)).find((value) => value !== undefined) ?? [];

    // open a suite's inline callback to the declarations nested in it
    if (kind === "suite") {
        const callback = node.arguments.at(-1);
        const isInline =
            callback?.type === "ArrowFunctionExpression" || callback?.type === "FunctionExpression";
        if (callback === undefined || !isInline) {
            throw BuildError.of(
                "INSPECTION_INVALID",
                `suite declaration at '${location}' is invalid: it requires an inline callback`,
            );
        }
        suites.set(callback, [...titles, name]);
    }

    return { kind, name, suites: titles, file, start: node.start, end: node.end, modifiers };
}

/** Read a string literal's or a substitution-free template's text, absent for any other node. */
function literalText(node: ESTree.Node): string | undefined {
    if (node.type === "Literal" && typeof node.value === "string") {
        return node.value;
    } else if (node.type === "TemplateLiteral" && node.expressions.length === 0) {
        return node.quasis[0]?.value.cooked ?? undefined;
    }

    return undefined;
}

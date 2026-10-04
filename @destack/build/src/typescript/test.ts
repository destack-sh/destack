import type { Project } from "typescript/unstable/async";
import {
    type CallExpression,
    type Identifier,
    isArrowFunction,
    isCallExpression,
    isFunctionDeclaration,
    isFunctionExpression,
    isIdentifier,
    isImportDeclaration,
    isNamedImports,
    isNoSubstitutionTemplateLiteral,
    isPropertyAccessExpression,
    isStringLiteral,
    type Node,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import type { TestDeclaration } from "@destack/test/inspect";
import { BuildError } from "../error/index.ts";

/** The statements and expressions that register tests only on some executions. */
const CONDITIONAL_KINDS = new Set([
    SyntaxKind.IfStatement,
    SyntaxKind.ForStatement,
    SyntaxKind.ForOfStatement,
    SyntaxKind.ForInStatement,
    SyntaxKind.WhileStatement,
    SyntaxKind.DoStatement,
    SyntaxKind.ConditionalExpression,
    SyntaxKind.SwitchStatement,
    SyntaxKind.BinaryExpression,
]);

/** A call registering a test or suite, with the modifiers between it and the called name. */
interface TestCall {
    /** The outermost call. */
    readonly node: CallExpression;
    /** The called name. */
    readonly expression: Identifier;
    /** The modifiers, such as `skip` or `each`, outermost last. */
    readonly modifiers: string[];
}

/** Collect imported test declarations without evaluating their arguments or bodies. */
export async function collectTests(
    source: SourceFile,
    file: string,
    project: Project,
): Promise<TestDeclaration[]> {
    // skip files without test imports
    const bindings = await testBindings(source, project);
    if (!bindings.size) {
        return [];
    }

    // resolve the called names in one compiler request
    const calls = collectCalls(source);
    const symbols = calls.length
        ? await project.checker.getSymbolAtLocation(calls.map((call) => call.expression))
        : [];

    // collect literal declarations in module scope and suite callbacks, in source order
    const declarations: TestDeclaration[] = [];
    const suites = new Set<Node>();
    for (const [index, call] of calls.entries()) {
        // resolve the declaration kind through its imported symbol
        const symbol = symbols[index];
        const kind = symbol && bindings.get(symbol.id);
        if (kind) {
            const declaration = describeTest(call, kind, file, suites);
            if (declaration !== undefined) {
                declarations.push(declaration);
            }
        }
    }

    return declarations.toSorted((left, right) => left.start - right.start);
}

/** Map the test and suite functions a module imports by symbol, so local shadowing cannot match. */
async function testBindings(
    source: SourceFile,
    project: Project,
): Promise<Map<number, "test" | "suite">> {
    // read the imports of each statement
    const bindings = new Map<number, "test" | "suite">();
    for (const statement of source.statements) {
        // read named imports of the test packages
        if (!isImportDeclaration(statement) || !isStringLiteral(statement.moduleSpecifier)) {
            continue;
        }
        if (!["@destack/test", "vitest"].includes(statement.moduleSpecifier.text)) {
            continue;
        }
        const imported = statement.importClause?.namedBindings;
        if (!imported || !isNamedImports(imported)) {
            continue;
        }

        // retain named test and suite imports by symbol
        for (const binding of imported.elements) {
            const name = (binding.propertyName ?? binding.name).text;
            if (!["test", "it", "describe", "suite"].includes(name)) {
                continue;
            }
            const symbol = await project.checker.getSymbolAtLocation(binding.name);
            if (symbol) {
                bindings.set(symbol.id, name === "test" || name === "it" ? "test" : "suite");
            }
        }
    }

    return bindings;
}

/** Collect the outermost calls and the names they call through modifiers and parameterized calls. */
function collectCalls(source: SourceFile): TestCall[] {
    // walk the module breadth first
    const calls: TestCall[] = [];
    const pending: Node[] = [...source.statements];
    for (const node of pending) {
        // visit every node, keeping outermost calls
        node.forEachChild((child) => {
            pending.push(child);
        });
        if (!isCallExpression(node)) {
            continue;
        }
        if (isCallExpression(node.parent) && node.parent.expression === node) {
            continue;
        }

        // follow modifiers and parameterized calls to the called name
        let expression = node.expression;
        const modifiers: string[] = [];
        while (isPropertyAccessExpression(expression) || isCallExpression(expression)) {
            if (isPropertyAccessExpression(expression)) {
                modifiers.unshift(expression.name.text);
            }
            expression = expression.expression;
        }
        if (isIdentifier(expression)) {
            calls.push({ node, expression, modifiers });
        }
    }

    return calls;
}

/** Describe a test registration in module or suite scope, absent for one inside another function. */
function describeTest(
    call: TestCall,
    kind: "test" | "suite",
    file: string,
    suites: Set<Node>,
): TestDeclaration | undefined {
    // leave registrations in other functions to the modules calling them
    const { node, modifiers } = call;
    const scopes: Node[] = [];
    for (let parent = node.parent; parent.kind !== SyntaxKind.SourceFile; parent = parent.parent) {
        scopes.push(parent);
    }
    if (scopes.some((scope) => isFunction(scope) && !suites.has(scope))) {
        return undefined;
    }

    // reject registrations whose presence depends on executing application code
    const location = `${file}:${node.getStart()}`;
    if (scopes.some((scope) => CONDITIONAL_KINDS.has(scope.kind))) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `test declaration requires module or suite scope: ${location}`,
        );
    }

    // require a title that can be read without evaluation
    const title = node.arguments[0];
    if (!title || !(isStringLiteral(title) || isNoSubstitutionTemplateLiteral(title))) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `test declaration requires a literal title: ${location}`,
        );
    }

    // allow nested declarations within the suite callback
    if (kind === "suite") {
        const callback = node.arguments.at(-1);
        if (!callback || !(isArrowFunction(callback) || isFunctionExpression(callback))) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `suite declaration requires an inline callback: ${location}`,
            );
        }
        suites.add(callback);
    }

    // retain parameterized cases as declarations with their literal title patterns
    return {
        kind,
        name: title.text,
        file,
        start: node.getStart(),
        end: node.getEnd(),
        modifiers,
    };
}

/** Report whether a node is a function body that runs when called. */
function isFunction(node: Node): boolean {
    return isArrowFunction(node) || isFunctionExpression(node) || isFunctionDeclaration(node);
}

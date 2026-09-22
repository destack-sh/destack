import type { Project } from "typescript/unstable/async";
import {
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

/** Collect imported test declarations without evaluating their arguments or bodies. */
export async function collectTests(
    source: SourceFile,
    file: string,
    project: Project,
): Promise<TestDeclaration[]> {
    const bindings = new Map<number, "test" | "suite">();

    // identify imports by compiler symbols so local shadowing cannot create false matches
    for (const statement of source.statements) {
        if (!isImportDeclaration(statement) || !isStringLiteral(statement.moduleSpecifier)) {
            continue;
        }
        if (!["@destack/test", "vitest"].includes(statement.moduleSpecifier.text)) {
            continue;
        }

        // retain named test and suite imports
        const imported = statement.importClause?.namedBindings;
        if (!imported || !isNamedImports(imported)) {
            continue;
        }
        for (const binding of imported.elements) {
            const name = (binding.propertyName ?? binding.name).text;
            if (!["test", "it", "describe", "suite"].includes(name)) {
                continue;
            }

            // distinguish imported functions from shadowed local names
            const symbol = await project.checker.getSymbolAtLocation(binding.name);
            if (symbol) {
                bindings.set(symbol.id, name === "test" || name === "it" ? "test" : "suite");
            }
        }
    }

    if (!bindings.size) {
        return [];
    }

    // collect literal declarations in module scope and suite callbacks
    const declarations: TestDeclaration[] = [];
    const suites = new Set<Node>();
    const pending: Node[] = [...source.statements];
    for (const node of pending) {
        node.forEachChild((child) => {
            pending.push(child);
        });
        if (!isCallExpression(node)) {
            continue;
        }
        if (node.parent && isCallExpression(node.parent) && node.parent.expression === node) {
            continue;
        }

        // follow modifiers and parameterized calls to the imported function
        let expression = node.expression;
        const modifiers: string[] = [];
        while (isPropertyAccessExpression(expression) || isCallExpression(expression)) {
            if (isPropertyAccessExpression(expression)) {
                modifiers.unshift(expression.name.text);
                expression = expression.expression;
            } else {
                expression = expression.expression;
            }
        }
        if (!isIdentifier(expression)) {
            continue;
        }

        // resolve the declaration kind through its imported symbol
        const symbol = await project.checker.getSymbolAtLocation(expression);
        const kind = symbol && bindings.get(symbol.id);
        if (!kind) {
            continue;
        }

        // require a title that can be read without evaluation
        const title = node.arguments[0];
        const name =
            title && (isStringLiteral(title) || isNoSubstitutionTemplateLiteral(title))
                ? title.text
                : undefined;
        if (name === undefined) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `Test declaration requires a literal title: ${file}:${node.getStart()}`,
            );
        }

        // reject registrations whose presence depends on executing application code
        let parent = node.parent;
        while (parent && parent.kind !== SyntaxKind.SourceFile) {
            if (
                [
                    SyntaxKind.IfStatement,
                    SyntaxKind.ForStatement,
                    SyntaxKind.ForOfStatement,
                    SyntaxKind.ForInStatement,
                    SyntaxKind.WhileStatement,
                    SyntaxKind.DoStatement,
                    SyntaxKind.ConditionalExpression,
                    SyntaxKind.SwitchStatement,
                    SyntaxKind.BinaryExpression,
                ].includes(parent.kind) ||
                ((isArrowFunction(parent) ||
                    isFunctionExpression(parent) ||
                    isFunctionDeclaration(parent)) &&
                    !suites.has(parent))
            ) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `Test declaration requires module or suite scope: ${file}:${node.getStart()}`,
                );
            }
            parent = parent.parent;
        }

        // allow nested declarations within the suite callback
        if (kind === "suite") {
            const callback = node.arguments.at(-1);
            if (!callback || !(isArrowFunction(callback) || isFunctionExpression(callback))) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `Suite declaration requires an inline callback: ${file}:${node.getStart()}`,
                );
            }
            suites.add(callback);
        }

        // retain parameterized cases as declarations with their literal title patterns
        declarations.push({
            kind,
            name,
            file,
            start: node.getStart(),
            end: node.getEnd(),
            modifiers,
        });
    }

    return declarations.sort((left, right) => left.start - right.start);
}

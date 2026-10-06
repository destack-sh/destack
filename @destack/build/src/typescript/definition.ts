import type { Symbol as TypeScriptSymbol } from "typescript/unstable/async";
import {
    type CallExpression,
    type Expression,
    isArrayLiteralExpression,
    isCallExpression,
    isIdentifier,
    isImportDeclaration,
    isMethodDeclaration,
    isNamedImports,
    isNoSubstitutionTemplateLiteral,
    isNumericLiteral,
    isObjectLiteralExpression,
    isPrefixUnaryExpression,
    isPropertyAccessExpression,
    isPropertyAssignment,
    isStringLiteral,
    isVariableDeclaration,
    type Node,
    NodeFlags,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import type { JsonValue } from "@destack/schema";
import type { SourceRange } from "@destack/package/source";
import { BuildError } from "../error/index.ts";
import { exportedNames } from "./declaration.ts";
import type { SymbolInspector } from "./symbol.ts";

/** The module declaring the example and scenario constructors. */
const DECLARE_MODULE = "@destack/package/declare";

/** Where a module defines a declaration: its literal name, the exported constant holding it and its defining call. */
export interface DefinitionSite {
    /** The literal name. */
    readonly name: string;
    /** The exported constant holding the declaration. */
    readonly symbol: string;
    /** The defining call's range in its module. */
    readonly source: SourceRange;
}

/** A constructor call initializing an exported module constant, with its site and its definition's fields. */
export interface Definition {
    /** Where the module defines the declaration. */
    readonly site: DefinitionSite;
    /** The fields of the definition the call takes. */
    readonly fields: DefinitionFields;
}

/** Locate a module's calls of a constructor of `@destack/package/declare` at exported module constants. */
export async function locateDefinitions(
    source: SourceFile,
    file: string,
    inspector: SymbolInspector,
    constructor: string,
    kind: string,
): Promise<Definition[]> {
    // skip modules that import no such constructor
    const bindings = await constructorBindings(source, inspector, constructor);
    if (!bindings.size) {
        return [];
    }

    // resolve every call's callee in one compiler request, and the module's exports
    const exported = await exportedNames(source, inspector);
    const calls = collectCalls(source);
    const callees = await inspector.project.checker.getSymbolAtLocation(
        calls.map((call) => call.expression),
    );

    // keep the constructor's calls at exported module constants
    const definitions: Definition[] = [];
    for (const [index, call] of calls.entries()) {
        const callee = callees[index];
        if (callee !== undefined && bindings.has(callee.id)) {
            const symbol = await locateConstant(call, kind, file, inspector, exported);
            const fields = new DefinitionFields(call, kind, `${file}#${symbol}`);
            const range = { file, start: call.getStart(), end: call.getEnd() };
            definitions.push({
                site: { name: fields.text("name"), symbol, source: range },
                fields,
            });
        }
    }

    return definitions;
}

/** The fields of a definition's object literal, read without evaluating them. */
export class DefinitionFields {
    /** The object literal's fields by name. */
    readonly #fields = new Map<string, Expression>();
    /** The declaration kind, for errors. */
    readonly #kind: string;
    /** The declaring symbol, for errors. */
    readonly location: string;

    /** Read the fields of a call's first argument or of an object literal. */
    constructor(node: CallExpression | Expression, kind: string, location: string) {
        // require an object literal of named fields
        this.#kind = kind;
        this.location = location;
        const literal = isCallExpression(node) ? node.arguments[0] : node;
        if (literal === undefined || !isObjectLiteralExpression(literal)) {
            throw this.#error("an object literal");
        }
        for (const property of literal.properties) {
            // keep named fields, and leave methods such as render to the runtime
            if (isPropertyAssignment(property) && isFieldName(property.name)) {
                this.#fields.set(property.name.text, property.initializer);
            } else if (!isMethodDeclaration(property)) {
                throw this.#error("named fields without shorthands or spreads");
            }
        }
    }

    /** Read a required literal string field. */
    text(name: string): string {
        const value = this.literal(name);
        if (typeof value !== "string") {
            throw this.#error(`a literal ${name}`);
        }

        return value;
    }

    /** Read a field as an object literal. */
    object(name: string): Expression {
        const field = this.#fields.get(name);
        if (field === undefined || !isObjectLiteralExpression(field)) {
            throw this.#error(`an object literal ${name}`);
        }

        return field;
    }

    /** Read a field mapping names to lists of object literals, such as an example's calls by scope, none when absent. */
    lists(name: string): Map<string, DefinitionFields[]> {
        // read nothing for an absent field
        const field = this.#fields.get(name);
        if (field === undefined) {
            return new Map();
        }

        // read each name's array of object literals
        const lists = new Map<string, DefinitionFields[]>();
        for (const [key, value] of new DefinitionFields(
            this.object(name),
            this.#kind,
            this.location,
        ).#entries()) {
            if (!isArrayLiteralExpression(value)) {
                throw this.#error(`an array literal ${name}.${key}`);
            }
            lists.set(
                key,
                value.elements.map(
                    (element) => new DefinitionFields(element, this.#kind, this.location),
                ),
            );
        }

        return lists;
    }

    /** Read an optional field as literal JSON. */
    literal(name: string): JsonValue | undefined {
        const field = this.#fields.get(name);

        return field === undefined ? undefined : this.#json(field, name);
    }

    /** Resolve a field naming a symbol, such as an imported component. */
    async symbol(name: string, inspector: SymbolInspector): Promise<TypeScriptSymbol> {
        const field = this.#fields.get(name);
        if (field === undefined) {
            throw this.#error(`a ${name}`);
        }

        return await this.#resolve(field, name, inspector);
    }

    /** Resolve a field listing symbols in an array literal. */
    async symbols(name: string, inspector: SymbolInspector): Promise<TypeScriptSymbol[]> {
        const field = this.#fields.get(name);
        if (field === undefined || !isArrayLiteralExpression(field)) {
            throw this.#error(`an array literal ${name}`);
        }

        return await Promise.all(
            field.elements.map((element) => this.#resolve(element, name, inspector)),
        );
    }

    /** Resolve a name or property access to its original symbol. */
    async #resolve(
        node: Expression,
        name: string,
        inspector: SymbolInspector,
    ): Promise<TypeScriptSymbol> {
        // require a name the compiler resolves
        const isName = isIdentifier(node) || isPropertyAccessExpression(node);
        const symbol = isName
            ? await inspector.project.checker.getSymbolAtLocation(node)
            : undefined;
        if (symbol === undefined) {
            throw this.#error(`${name} to name a declared symbol`);
        }

        return await inspector.original(symbol);
    }

    /** Read a literal expression as JSON. */
    #json(node: Expression, name: string): JsonValue {
        // read strings, numbers, booleans and null
        if (isStringLiteral(node) || isNoSubstitutionTemplateLiteral(node)) {
            return node.text;
        } else if (isNumericLiteral(node)) {
            return Number(node.text);
        } else if (
            isPrefixUnaryExpression(node) &&
            node.operator === SyntaxKind.MinusToken &&
            isNumericLiteral(node.operand)
        ) {
            return -Number(node.operand.text);
        } else if (node.kind === SyntaxKind.TrueKeyword || node.kind === SyntaxKind.FalseKeyword) {
            return node.kind === SyntaxKind.TrueKeyword;
        } else if (node.kind === SyntaxKind.NullKeyword) {
            return null;
        }
        // read arrays and objects field by field
        else if (isArrayLiteralExpression(node)) {
            return node.elements.map((element) => this.#json(element, name));
        } else if (isObjectLiteralExpression(node)) {
            return Object.fromEntries(
                new DefinitionFields(node, this.#kind, this.location).#entries().map(
                    ([key, value]) => [key, this.#json(value, name)],
                ),
            );
        }
        // refuse anything that needs evaluation
        else {
            throw this.#error(`${name} as literal JSON`);
        }
    }

    /** List the fields in source order. */
    #entries(): [string, Expression][] {
        return [...this.#fields];
    }

    /** Describe a definition that cannot be read without evaluation. */
    #error(requirement: string): BuildError {
        return new BuildError(
            "INSPECTION_FAILED",
            `${this.#kind} ${this.location} requires ${requirement}`,
        );
    }
}

/** Map a constructor's imports by symbol, so local shadowing cannot match. */
async function constructorBindings(
    source: SourceFile,
    inspector: SymbolInspector,
    constructor: string,
): Promise<Set<number>> {
    const bindings = new Set<number>();
    for (const statement of source.statements) {
        // read named imports of the constructors' module
        if (
            !isImportDeclaration(statement) ||
            !isStringLiteral(statement.moduleSpecifier) ||
            statement.moduleSpecifier.text !== DECLARE_MODULE
        ) {
            continue;
        }
        const imported = statement.importClause?.namedBindings;
        if (!imported || !isNamedImports(imported)) {
            continue;
        }

        // retain each binding of the constructor by symbol
        for (const binding of imported.elements) {
            const isConstructor = (binding.propertyName ?? binding.name).text === constructor;
            const symbol = isConstructor
                ? await inspector.project.checker.getSymbolAtLocation(binding.name)
                : undefined;
            if (symbol !== undefined) {
                bindings.add(symbol.id);
            }
        }
    }

    return bindings;
}

/** Collect a module's calls. */
function collectCalls(source: SourceFile): CallExpression[] {
    // walk the module breadth first, keeping every call
    const calls: CallExpression[] = [];
    const pending: Node[] = [...source.statements];
    for (const node of pending) {
        node.forEachChild((child) => {
            pending.push(child);
        });
        if (isCallExpression(node)) {
            calls.push(node);
        }
    }

    return calls;
}

/** Require a constructor call to initialize an exported module constant, returning the constant's name. */
async function locateConstant(
    call: CallExpression,
    kind: string,
    file: string,
    inspector: SymbolInspector,
    exported: ReadonlyMap<number, string>,
): Promise<string> {
    // require a named module constant
    const declaration = call.parent;
    const isConstant =
        isVariableDeclaration(declaration) &&
        isIdentifier(declaration.name) &&
        Boolean(declaration.parent.flags & NodeFlags.Const) &&
        declaration.parent.parent.parent.kind === SyntaxKind.SourceFile;
    if (!isConstant || !isIdentifier(declaration.name)) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `${kind} in ${file} must initialize a module-level constant`,
        );
    }

    // require the constant to be exported
    const symbol = await inspector.project.checker.getSymbolAtLocation(declaration.name);
    if (symbol === undefined || !exported.has(symbol.id)) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `export ${file}#${declaration.name.text} to inspect its ${kind}`,
        );
    }

    return declaration.name.text;
}

/** Report whether a property name is one a literal field can carry. */
function isFieldName(name: Node): name is Node & { readonly text: string } {
    return isIdentifier(name) || isStringLiteral(name);
}

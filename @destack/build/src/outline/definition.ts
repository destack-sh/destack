import type { ESTree } from "rolldown/utils";
import type { FamilyError } from "@destack/error";
import type { graph } from "@destack/package";
import type { SourceRange } from "@destack/package/source";
import { JsonValue } from "@destack/schema";
import { BuildError, type BuildReason } from "../error/index.ts";
import { children, qualifiedName } from "./node.ts";

/** The module declaring the example and scenario constructors. */
const DECLARE_MODULE = "@destack/package/declare";

/** The declaration kinds a module defines through constructor calls. */
type DefinitionKind = "example" | "scenario";

/** The constructors whose calls a module's outline reads, by the declaration kind each defines. */
const CONSTRUCTORS: Readonly<Record<string, DefinitionKind>> = {
    defineExample: "example",
    defineScenario: "scenario",
};

/** Resolve a name a module uses to its symbol, absent for one it neither declares nor imports. */
export type NameResolver = (name: readonly string[]) => Promise<graph.Moniker | undefined>;

/** Where a module defines a declaration: its literal name, constant and defining call. */
export interface DefinitionSite {
    /** The literal name. */
    readonly name: string;
    /** The exported constant with the declaration. */
    readonly symbol: string;
    /** The defining call's range in its module. */
    readonly source: SourceRange;
}

/** A constructor call initializing an exported constant: its kind, site and fields. */
export interface Definition {
    /** The declaration kind its constructor defines. */
    readonly kind: DefinitionKind;
    /** Where the module defines it. */
    readonly site: DefinitionSite;
    /** The fields of the definition the call takes. */
    readonly fields: DefinitionFields;
}

/** Collect a module's example and scenario constructor calls at its exported constants. */
export function collectDefinitions(
    program: ESTree.Program,
    file: string,
    exported: ReadonlySet<string>,
    resolve: NameResolver,
): Definition[] {
    // bind the constructors and the namespaces holding them the module imports, by local name
    const constructors = new Map<string, DefinitionKind>();
    const namespaces = new Set<string>();
    for (const statement of program.body) {
        if (statement.type === "ImportDeclaration" && statement.source.value === DECLARE_MODULE) {
            for (const specifier of statement.specifiers) {
                const kind =
                    specifier.type === "ImportSpecifier" && specifier.imported.type === "Identifier"
                        ? CONSTRUCTORS[specifier.imported.name]
                        : undefined;
                if (specifier.type === "ImportNamespaceSpecifier") {
                    namespaces.add(specifier.local.name);
                } else if (kind !== undefined) {
                    constructors.set(specifier.local.name, kind);
                }
            }
        }
    }
    if (constructors.size === 0 && namespaces.size === 0) {
        return [];
    }
    const bound = { constructors, namespaces };

    // read the module's constants, whose literal values fields may name
    const constants = new Map<string, ESTree.Node>();
    for (const statement of program.body) {
        const declaration =
            statement.type === "ExportNamedDeclaration" ? statement.declaration : statement;
        if (declaration?.type === "VariableDeclaration" && declaration.kind === "const") {
            for (const { id, init } of declaration.declarations) {
                if (id.type === "Identifier" && init !== null) {
                    constants.set(id.name, init);
                }
            }
        }
    }

    // take each constructor call initializing a module-level constant
    const definitions: Definition[] = [];
    const taken = new Set<ESTree.Node>();
    for (const statement of program.body) {
        const declaration =
            statement.type === "ExportNamedDeclaration" ? statement.declaration : statement;
        if (declaration?.type !== "VariableDeclaration" || declaration.kind !== "const") {
            continue;
        }
        for (const declarator of declaration.declarations) {
            // take a call of a bound constructor at a constant's name
            const { init, id } = declarator;
            const kind = init === null ? undefined : constructorKind(init, bound);
            if (kind === undefined || init?.type !== "CallExpression" || id.type !== "Identifier") {
                continue;
            }
            taken.add(init);

            // require an exported constant
            if (!exported.has(id.name)) {
                throw BuildError.of(
                    "DECLARATION_INVALID",
                    `declared '${id.name}' is invalid: '${file}' does not export it`,
                );
            }

            // read its fields and site
            const location = `${file}#${id.name}`;
            const fields = DefinitionFields.of(init, { kind, location, constants, resolve });
            const source = { file, start: init.start, end: init.end };
            definitions.push({
                kind,
                site: { name: fields.text("name"), symbol: id.name, source },
                fields,
            });
        }
    }

    // refuse a constructor call anywhere else
    const pending: ESTree.Node[] = [program];
    for (let node = pending.pop(); node !== undefined; node = pending.pop()) {
        const kind = taken.has(node) ? undefined : constructorKind(node, bound);
        if (kind !== undefined) {
            throw BuildError.of(
                "INSPECTION_INVALID",
                `definition ${kind} at '${file}:${String(node.start)}' is invalid: it must initialize a module-level constant`,
            );
        }
        pending.push(...children(node));
    }

    return definitions;
}

/** The fields of a definition's object literal, read without evaluating the module. */
export class DefinitionFields {
    /** The object literal's fields by name. */
    readonly #fields: ReadonlyMap<string, ESTree.Node>;
    /** The module's top-level constants, whose literal values fields may name. */
    readonly #constants: ReadonlyMap<string, ESTree.Node>;
    /** The declaring symbol, for errors. */
    readonly location: string;
    /** The declaration kind, for errors. */
    readonly #kind: DefinitionKind;
    /** Resolves the names fields refer to. */
    readonly #resolve: NameResolver;

    /** Read the fields of an object literal. */
    private constructor(
        fields: ReadonlyMap<string, ESTree.Node>,
        constants: ReadonlyMap<string, ESTree.Node>,
        kind: DefinitionKind,
        location: string,
        resolve: NameResolver,
    ) {
        // store the fields, the constants and what errors and names resolve against
        this.#fields = fields;
        this.#constants = constants;
        this.#kind = kind;
        this.location = location;
        this.#resolve = resolve;
    }

    /** Read the fields of a constructor call's first argument. */
    static of(
        call: ESTree.CallExpression,
        context: {
            readonly kind: DefinitionKind;
            readonly location: string;
            readonly constants: ReadonlyMap<string, ESTree.Node>;
            readonly resolve: NameResolver;
        },
    ): DefinitionFields {
        const { kind, location, constants, resolve } = context;
        const empty = new DefinitionFields(new Map(), constants, kind, location, resolve);

        return empty.#nested(call.arguments[0]);
    }

    /** Read a required literal string field. */
    text(name: string): string {
        const value = this.literal(name);
        if (typeof value !== "string") {
            throw this.#error(`a literal ${name}`);
        }

        return value;
    }

    /** Read a field's object literal as fields of its own. */
    object(name: string): DefinitionFields {
        return this.#nested(this.#fields.get(name));
    }

    /** Read a field of keyed object literal lists, such as an example's calls by scope. */
    lists(name: string): Map<string, DefinitionFields[]> {
        // read nothing for an absent field
        const field = this.#fields.get(name);
        if (field === undefined) {
            return new Map();
        }

        // read each key's array of object literals
        const lists = new Map<string, DefinitionFields[]>();
        for (const [key, value] of this.object(name).#fields) {
            if (value.type !== "ArrayExpression") {
                throw this.#error(`an array literal ${name}.${key}`);
            }
            lists.set(
                key,
                value.elements.map((element) => this.#nested(element ?? undefined)),
            );
        }

        return lists;
    }

    /** Read an optional field as literal JSON. */
    literal(name: string): JsonValue | undefined {
        const field = this.#fields.get(name);

        return field === undefined ? undefined : this.#json(field, name);
    }

    /** Resolve a field naming a symbol, such as an imported component, to its moniker. */
    async symbol(name: string): Promise<graph.Moniker> {
        const field = this.#fields.get(name);
        if (field === undefined) {
            throw this.#error(`a ${name}`);
        }

        return await this.#symbolOf(field, name);
    }

    /** Resolve a field listing symbols in an array literal to their monikers. */
    async symbols(name: string): Promise<graph.Moniker[]> {
        const field = this.#fields.get(name);
        if (field?.type !== "ArrayExpression") {
            throw this.#error(`an array literal ${name}`);
        }

        return await Promise.all(
            field.elements.map((element) =>
                element === null
                    ? Promise.reject(this.#error(name))
                    : this.#symbolOf(element, name),
            ),
        );
    }

    /** Resolve a name or member access to the symbol it names. */
    async #symbolOf(node: ESTree.Node, name: string): Promise<graph.Moniker> {
        // resolve its parts through the module, refusing a name it does not resolve
        const parts = qualifiedName(node);
        const resolved = parts === undefined ? undefined : await this.#resolve(parts);
        if (resolved === undefined) {
            throw this.#error(`${name} to name a declared symbol`);
        }

        return resolved;
    }

    /** Read an object literal of named fields, leaving methods such as render to the runtime. */
    #nested(node: ESTree.Node | undefined): DefinitionFields {
        // require an object literal
        if (node?.type !== "ObjectExpression") {
            throw this.#error("an object literal");
        }

        // take each named field, and the fields of a spread constant in its place
        const fields = new Map<string, ESTree.Node>();
        for (const property of node.properties) {
            const key =
                property.type === "Property" && !property.computed
                    ? propertyName(property.key)
                    : undefined;
            const isMethod = property.type === "Property" && property.method;
            if (property.type === "SpreadElement") {
                for (const [spreadKey, value] of this.#nested(
                    this.#constant(property.argument),
                ).#fields) {
                    fields.set(spreadKey, value);
                }
            } else if (property.type === "Property" && key !== undefined && !isMethod) {
                fields.set(key, property.value);
            } else if (!isMethod) {
                throw this.#error("named fields without computed keys");
            }
        }

        return new DefinitionFields(
            fields,
            this.#constants,
            this.#kind,
            this.location,
            this.#resolve,
        );
    }

    /** Read a literal expression as JSON, through assertions and the module's constants. */
    #json(node: ESTree.Node, name: string): JsonValue {
        // read strings, numbers, booleans and null
        if (
            node.type === "Literal" &&
            (typeof node.value === "string" ||
                typeof node.value === "number" ||
                typeof node.value === "boolean" ||
                node.value === null)
        ) {
            return node.value;
        } else if (node.type === "TemplateLiteral" && node.expressions.length === 0) {
            return node.quasis[0]?.value.cooked ?? "";
        } else if (
            node.type === "UnaryExpression" &&
            node.operator === "-" &&
            node.argument.type === "Literal" &&
            typeof node.argument.value === "number"
        ) {
            return -node.argument.value;
        }
        // read arrays element by element, splicing in the elements of a spread constant
        else if (node.type === "ArrayExpression") {
            return node.elements.flatMap((element): readonly JsonValue[] => {
                if (element?.type === "SpreadElement") {
                    const spread = this.#json(element.argument, name);
                    if (!JsonValue.isArray(spread)) {
                        throw this.#error(`${name} as literal JSON`);
                    }

                    return spread;
                }

                return [element === null ? null : this.#json(element, name)];
            });
        } else if (node.type === "ObjectExpression") {
            return Object.fromEntries(
                [...this.#nested(node).#fields].map(([key, value]) => [
                    key,
                    this.#json(value, name),
                ]),
            );
        }
        // read through type assertions, which leave the value alone
        else if (node.type === "TSAsExpression" || node.type === "TSSatisfiesExpression") {
            return this.#json(node.expression, name);
        }
        // read a module constant's literal value, as constants name the literals they hold
        else if (node.type === "Identifier") {
            const value = this.#constants.get(node.name);
            if (value !== undefined) {
                return this.#json(value, name);
            }
        }

        // refuse anything that needs evaluation
        throw this.#error(`${name} as literal JSON`);
    }

    /** Read the literal a module constant holds, through type assertions. */
    #constant(node: ESTree.Node): ESTree.Node {
        // follow a constant's name to its value, unwrapping assertions
        if (node.type === "Identifier") {
            const value = this.#constants.get(node.name);

            return value === undefined ? node : this.#constant(value);
        } else if (node.type === "TSAsExpression" || node.type === "TSSatisfiesExpression") {
            return this.#constant(node.expression);
        }

        return node;
    }

    /** Describe a definition the reader cannot read. */
    #error(requirement: string): FamilyError<BuildReason> {
        return BuildError.of(
            "INSPECTION_INVALID",
            `definition ${this.#kind} '${this.location}' is invalid: it requires '${requirement}'`,
        );
    }
}

/** Read a property key's name, absent for a computed one. */
function propertyName(key: ESTree.Node): string | undefined {
    if (key.type === "Identifier") {
        return key.name;
    } else if (key.type === "Literal" && typeof key.value === "string") {
        return key.value;
    }

    return undefined;
}

/** Read the definition kind of a call of a bound constructor, absent for any other node. */
function constructorKind(
    node: ESTree.Node,
    bound: {
        readonly constructors: ReadonlyMap<string, DefinitionKind>;
        readonly namespaces: ReadonlySet<string>;
    },
): DefinitionKind | undefined {
    // name the callee, a constructor or a member of a namespace holding them
    const name = node.type === "CallExpression" ? qualifiedName(node.callee) : undefined;
    const [head, member, ...rest] = name ?? [];
    if (head === undefined || rest.length > 0) {
        return undefined;
    }

    return member === undefined
        ? bound.constructors.get(head)
        : bound.namespaces.has(head)
          ? CONSTRUCTORS[member]
          : undefined;
}

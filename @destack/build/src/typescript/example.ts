import { Control, ControlInput, ExampleDescription } from "@destack/package/declare";
import {
    SignatureKind,
    SymbolFlags,
    type Symbol as TypeScriptSymbol,
    type Type,
    TypeFlags,
} from "typescript/unstable/async";
import {
    isLiteralTypeNode,
    isNoSubstitutionTemplateLiteral,
    isNumericLiteral,
    isParenthesizedTypeNode,
    isPropertySignatureDeclaration,
    isStringLiteral,
    isTypeAliasDeclaration,
    isTypeReferenceNode,
    isUnionTypeNode,
    type Node,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import type { SymbolReference } from "@destack/package/code";
import { type JsonObject, schema } from "@destack/schema";
import { isAuthored } from "../source/dependency.ts";
import { type DefinitionSite, locateDefinitions } from "./definition.ts";
import type { SymbolInspector } from "./symbol.ts";

/** A literal a select control offers. */
type Literal = string | number | boolean;

/** An example a module exports, located and described without evaluating the module. */
export interface ExampleDeclaration extends DefinitionSite {
    /** The symbol the example shows. */
    readonly of: SymbolReference;
    /** The calls bringing the shown declaration's objects into the example's state, by scope, each object type by symbol. */
    readonly objects: Readonly<
        Record<
            string,
            readonly {
                readonly object: SymbolReference;
                readonly method: string;
                readonly input: JsonObject;
            }[]
        >
    >;
    /** The description without its calls: the literal properties and the controls of the shown symbol. */
    readonly description: Omit<ExampleDescription, "objects">;
}

/** Collect a module's exported examples through the compiler, reading their literal fields. */
export async function collectExamples(
    source: SourceFile,
    file: string,
    inspector: SymbolInspector,
): Promise<ExampleDeclaration[]> {
    const definitions = await locateDefinitions(
        source,
        file,
        inspector,
        "defineExample",
        "example",
    );

    return await Promise.all(
        definitions.map(async ({ site, fields }) => {
            // describe the example by its properties and the controls of what it shows
            const of = await fields.symbol("of", inspector);
            const description = ExampleDescription.omit({ objects: true }).parse({
                description: fields.text("description"),
                properties: fields.literal("properties") ?? {},
                controls: await describeControls(of, inspector),
            });

            // read the calls bringing the shown declaration's objects into the example's state
            const objects: Record<string, ExampleDeclaration["objects"][string]> = {};
            for (const [scope, calls] of fields.lists("objects")) {
                objects[scope] = await Promise.all(
                    calls.map(async (call) => ({
                        object: await inspector.reference(await call.symbol("object", inspector)),
                        method: call.text("method"),
                        input: schema
                            .record(schema.string(), schema.json())
                            .parse(call.literal("input") ?? {}),
                    })),
                );
            }

            return { ...site, of: await inspector.reference(of), objects, description };
        }),
    );
}

/** Derive a control for each property the package declares on what a symbol renders with, such as a component's properties. */
async function describeControls(
    symbol: TypeScriptSymbol,
    inspector: SymbolInspector,
): Promise<Control[]> {
    // read the first parameter of the symbol's first call signature, none for a symbol no call renders
    const checker = inspector.project.checker;
    const type =
        symbol.flags & SymbolFlags.Value ? await checker.getTypeOfSymbol(symbol) : undefined;
    const [signature] =
        type === undefined ? [] : await checker.getSignaturesOfType(type, SignatureKind.Call);
    const [parameter] = signature === undefined ? [] : await signature.getParameters();
    const properties =
        parameter === undefined ? undefined : await checker.getTypeOfSymbol(parameter);
    if (properties === undefined) {
        return [];
    }

    // describe each property the package declares that a control can set
    const controls: Control[] = [];
    for (const property of await checker.getPropertiesOfType(properties)) {
        const handle = property.declarations[0];
        const declaration = handle === undefined ? undefined : await inspector.node(handle);
        if (
            declaration === undefined ||
            !isAuthored(inspector.root, declaration.getSourceFile().fileName)
        ) {
            continue;
        }
        const propertyType = await checker.getTypeOfSymbol(property);
        const spelled = isPropertySignatureDeclaration(declaration)
            ? await spelledLiterals(declaration.type, inspector)
            : [];
        const input = propertyType === undefined ? undefined : await inputOf(propertyType, spelled);
        if (input === undefined) {
            continue;
        }
        const label = (await inspector.documentation(property)).text.trim();
        controls.push({
            property: property.name,
            ...(label === "" ? {} : { label }),
            isOptional: Boolean(property.flags & SymbolFlags.Optional),
            input,
        });
    }

    return controls;
}

/** Read the input a control shows for a property's type, its options in the order the source spells them. */
async function inputOf(type: Type, spelled: readonly Literal[]): Promise<ControlInput | undefined> {
    // read the union's members other than undefined
    const members = (type.isUnionType() ? ((await type.getTypes()) ?? []) : [type]).filter(
        (member) => !(member.flags & TypeFlags.Undefined),
    );
    const literals = members.flatMap((member) =>
        member.isStringLiteralType() ||
        member.isNumberLiteralType() ||
        member.isBooleanLiteralType()
            ? [member.value]
            : [],
    );
    const [only] = members;

    // a switch for true and false
    if (literals.length === 2 && members.length === 2 && literals.every(isBoolean)) {
        return { kind: "boolean" };
    }
    // a select for a union of literals, ordered as spelled
    else if (literals.length >= 2 && literals.length === members.length) {
        const rank = (literal: Literal) => {
            const index = spelled.indexOf(literal);

            return index === -1 ? spelled.length : index;
        };

        return {
            kind: "select",
            options: literals.toSorted((left, right) => rank(left) - rank(right)),
        };
    }
    // a field for a string or a number
    else if (members.length === 1 && only !== undefined && only.flags & TypeFlags.String) {
        return { kind: "text" };
    } else if (members.length === 1 && only !== undefined && only.flags & TypeFlags.Number) {
        return { kind: "number" };
    }
    // no control for any other type
    else {
        return undefined;
    }
}

/** List the literals a type node spells in source order, following type aliases. */
async function spelledLiterals(
    node: Node | undefined,
    inspector: SymbolInspector,
): Promise<Literal[]> {
    // read a literal, a union's members in order, or a parenthesized type
    if (node === undefined) {
        return [];
    } else if (isLiteralTypeNode(node)) {
        const literal = node.literal;
        const value =
            isStringLiteral(literal) || isNoSubstitutionTemplateLiteral(literal)
                ? literal.text
                : isNumericLiteral(literal)
                  ? Number(literal.text)
                  : literal.kind === SyntaxKind.TrueKeyword ||
                      literal.kind === SyntaxKind.FalseKeyword
                    ? literal.kind === SyntaxKind.TrueKeyword
                    : undefined;

        return value === undefined ? [] : [value];
    } else if (isUnionTypeNode(node)) {
        const members = await Promise.all(
            node.types.map((member) => spelledLiterals(member, inspector)),
        );

        return members.flat();
    } else if (isParenthesizedTypeNode(node)) {
        return await spelledLiterals(node.type, inspector);
    }
    // follow a reference to the type alias it names
    else if (isTypeReferenceNode(node)) {
        const symbol = await inspector.project.checker.getSymbolAtLocation(node.typeName);
        const original = symbol === undefined ? undefined : await inspector.original(symbol);
        const handle = original?.declarations[0];
        const alias = handle === undefined ? undefined : await inspector.node(handle);

        return alias !== undefined && isTypeAliasDeclaration(alias)
            ? await spelledLiterals(alias.type, inspector)
            : [];
    }
    // spell nothing for any other type
    else {
        return [];
    }
}

/** Report whether a literal is a boolean. */
function isBoolean(value: Literal): value is boolean {
    return typeof value === "boolean";
}

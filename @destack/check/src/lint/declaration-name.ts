import type { ESTree, Rule } from "@oxlint/plugins";
import { isTestFile } from "./word.ts";

/** The declarations of something, by constructor: their export the camelCase of what they are of and their name. */
const OF = new Map([
    ["defineExample", "@destack/package"],
    ["defineScenario", "@destack/package"],
]);

/** The declarations named on their own, by constructor: their export the camelCase of their name. */
const OWN = new Map([["defineAuditAction", "@destack/audit"]]);

/** Require examples, scenarios and audit actions exported under the camelCase of their identity, examples and scenarios described. */
export const declarationName: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "export a declaration under its identity" },
        messages: {
            name: "[PK06] export this {{constructor}} as '{{expected}}', the camelCase of its identity",
            description: "[PK07] describe this {{constructor}} in one sentence",
        },
    },
    create(context) {
        // skip test modules
        if (isTestFile(context.filename)) {
            return {};
        }
        const constructors = new Map<string, string>();

        return {
            ImportDeclaration(node) {
                // record the constructors imported from the packages declaring them
                for (const specifier of node.specifiers) {
                    if (
                        specifier.type !== "ImportSpecifier" ||
                        specifier.imported.type !== "Identifier"
                    ) {
                        continue;
                    }
                    const imported = specifier.imported.name;
                    const owner = OF.get(imported) ?? OWN.get(imported);
                    if (owner !== undefined && node.source.value.startsWith(owner)) {
                        constructors.set(specifier.local.name, imported);
                    }
                }
            },
            CallExpression(node) {
                // read an exported definition of a recorded constructor
                const constructor =
                    node.callee.type === "Identifier"
                        ? constructors.get(node.callee.name)
                        : undefined;
                const definition = node.arguments[0];
                const declarator = node.parent;
                if (
                    constructor === undefined ||
                    definition?.type !== "ObjectExpression" ||
                    declarator?.type !== "VariableDeclarator" ||
                    declarator.id.type !== "Identifier"
                ) {
                    return;
                }

                // require the export its identity spells
                const isOf = OF.has(constructor);
                const expected = exportName(definition, isOf);
                if (expected !== undefined && declarator.id.name !== expected) {
                    context.report({ node, messageId: "name", data: { constructor, expected } });
                }

                // require a description of an example or scenario
                if (isOf && property(definition, "description") === undefined) {
                    context.report({ node, messageId: "description", data: { constructor } });
                }
            },
        };
    },
};

/** Spell the export of a definition's literal name and what it is of, absent for one it cannot read. */
function exportName(definition: ESTree.ObjectExpression, isOf: boolean): string | undefined {
    // read the literal name and the identifier it is of
    const name = property(definition, "name");
    const of = isOf ? property(definition, "of") : undefined;
    const ofName =
        of?.type === "Identifier"
            ? of.name
            : of?.type === "MemberExpression" && of.property.type === "Identifier"
              ? of.property.name
              : undefined;
    if (
        name?.type !== "Literal" ||
        typeof name.value !== "string" ||
        (isOf && ofName === undefined)
    ) {
        return undefined;
    }

    // join the words in camelCase
    const words = [...(ofName === undefined ? [] : [ofName]), ...name.value.split(/[.-]/u)];

    return words
        .map((word, index) =>
            index === 0
                ? `${word.charAt(0).toLowerCase()}${word.slice(1)}`
                : `${word.charAt(0).toUpperCase()}${word.slice(1)}`,
        )
        .join("");
}

/** Read the value of an object literal's property by key, absent when it has none. */
function property(object: ESTree.ObjectExpression, key: string): ESTree.Node | undefined {
    const found = object.properties.find(
        (entry) =>
            entry.type === "Property" && entry.key.type === "Identifier" && entry.key.name === key,
    );

    return found?.type === "Property" ? found.value : undefined;
}

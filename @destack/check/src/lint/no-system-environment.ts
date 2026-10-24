import type { ESTree, Rule } from "@oxlint/plugins";
import { isEntrypoint, isEnvironmentFile } from "./package.ts";

/** The environment package and its subpaths, whose nouns' system members are host effects. */
const ENVIRONMENT_SOURCE = /^@destack\/environment(?:\/|$)/u;

/** Reject the system effects outside entrypoints and the environment package. */
export const noSystemEnvironment: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "build the system environment in program entrypoints alone" },
        messages: {
            system: "take {{name}} from the environment the caller gives; only entrypoints read the system's",
        },
    },
    create(context) {
        // leave the environment package and program entrypoints
        if (
            isEnvironmentFile(context.filename) ||
            isEntrypoint(context.filename, context.settings)
        ) {
            return {};
        }

        /** Report whether a name reads a value, or a namespace, the environment package gives. */
        function isEnvironment(node: ESTree.Node, name: string): boolean {
            // walk the scopes outward to the name's declaration
            let scope: ReturnType<typeof context.sourceCode.getScope> | null =
                context.sourceCode.getScope(node);
            while (scope) {
                const definition = scope.set.get(name)?.defs[0];
                if (definition !== undefined) {
                    const declaration = definition.parent;

                    return (
                        definition.type === "ImportBinding" &&
                        declaration?.type === "ImportDeclaration" &&
                        declaration.importKind !== "type" &&
                        ENVIRONMENT_SOURCE.test(declaration.source.value)
                    );
                }
                scope = scope.upper;
            }

            return false;
        }

        /** Read the environment noun a member reads: a named import, or a namespace's. */
        function nounOf(node: ESTree.Node): string | undefined {
            // read a named import
            if (node.type === "Identifier") {
                return isEnvironment(node, node.name) ? node.name : undefined;
            }
            // read a noun of a namespace import
            else if (
                node.type === "MemberExpression" &&
                !node.computed &&
                node.object.type === "Identifier" &&
                node.property.type === "Identifier" &&
                isEnvironment(node, node.object.name)
            ) {
                return `${node.object.name}.${node.property.name}`;
            }
            // leave any other value
            else {
                return undefined;
            }
        }

        return {
            MemberExpression(node) {
                // report a system effect read through an environment noun, by name or computed
                const { property } = node;
                const isSystem = node.computed
                    ? property.type === "Literal" && property.value === "system"
                    : property.type === "Identifier" && property.name === "system";
                const noun = isSystem ? nounOf(node.object) : undefined;
                if (noun !== undefined) {
                    context.report({ node, messageId: "system", data: { name: `${noun}.system` } });
                }
            },
            VariableDeclarator(node) {
                // report a system effect a pattern takes from an environment noun
                const { id, init } = node;
                const noun =
                    id.type === "ObjectPattern" && init !== null ? nounOf(init) : undefined;
                const isSystem =
                    id.type === "ObjectPattern" &&
                    id.properties.some(
                        (property) =>
                            property.type === "Property" &&
                            property.key.type === "Identifier" &&
                            property.key.name === "system",
                    );
                if (noun !== undefined && isSystem) {
                    context.report({ node, messageId: "system", data: { name: `${noun}.system` } });
                }
            },
        };
    },
};

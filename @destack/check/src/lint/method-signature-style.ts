import type { Rule } from "@oxlint/plugins";

/** Require function members of interfaces and type literals as properties, checked strictly. */
export const methodSignatureStyle: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "declare function members as properties of a function type" },
        messages: {
            method: "declare '{{name}}' as a readonly property of a function type, whose parameters check strictly",
        },
    },
    create(context) {
        return {
            TSMethodSignature(node) {
                // allow accessors and members over their container's type parameters, which stay covariant
                const enclosing = typeParameters(context.sourceCode.getAncestors(node));
                const isEnclosingTyped = node.params.some((parameter) =>
                    mentions(parameter, enclosing),
                );
                if (node.kind !== "method" || isEnclosingTyped) {
                    return;
                }

                // report a method, whose parameters check bivariantly
                const name = node.key.type === "Identifier" ? node.key.name : "member";
                context.report({ node, messageId: "method", data: { name } });
            },
        };
    },
};

/** List the type parameter names of the interfaces and type aliases enclosing a member. */
function typeParameters(ancestors: readonly unknown[]): ReadonlySet<string> {
    const names = new Set<string>();
    for (const ancestor of ancestors) {
        // take the declaration's type parameters
        const kind = field(ancestor, "type");
        const isDeclaration =
            kind === "TSInterfaceDeclaration" || kind === "TSTypeAliasDeclaration";
        const declared = field(field(ancestor, "typeParameters"), "params");
        if (isDeclaration && Array.isArray(declared)) {
            for (const parameter of declared) {
                const name = field(field(parameter, "name"), "name") ?? field(parameter, "name");
                if (typeof name === "string") {
                    names.add(name);
                }
            }
        }
    }

    return names;
}

/** Decide whether a node's types refer to one of some type parameter names. */
function mentions(node: unknown, names: ReadonlySet<string>): boolean {
    // search a list's nodes, and match a reference by its name
    if (Array.isArray(node)) {
        return node.some((child) => mentions(child, names));
    } else if (typeof node !== "object" || node === null) {
        return false;
    }
    const name = field(field(node, "typeName"), "name");
    if (field(node, "type") === "TSTypeReference" && typeof name === "string" && names.has(name)) {
        return true;
    }

    // search the node's children
    return Object.entries(node).some(([key, child]) => key !== "parent" && mentions(child, names));
}

/** Read a field of a value, absent for a value without it. */
function field(value: unknown, key: string): unknown {
    return typeof value === "object" && value !== null && key in value
        ? Reflect.get(value, key)
        : undefined;
}

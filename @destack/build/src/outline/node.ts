import type { ESTree } from "rolldown/utils";

/** List a node's child nodes, in field order. */
export function children(node: ESTree.Node): ESTree.Node[] {
    const found: ESTree.Node[] = [];
    for (const [field, value] of Object.entries(node)) {
        // leave the link back to the parent
        if (field === "parent") {
            continue;
        }

        // take a node, and each node of a list
        const values: unknown[] = Array.isArray(value) ? value : [value];
        found.push(...values.filter(isNode));
    }

    return found;
}

/** Report whether a field's value is a syntax node. */
export function isNode(value: unknown): value is ESTree.Node {
    return (
        typeof value === "object" &&
        value !== null &&
        "type" in value &&
        typeof value.type === "string"
    );
}

/** Read an identifier, member access or qualified type name as its parts, absent for any other. */
export function qualifiedName(node: ESTree.Node): string[] | undefined {
    // name an identifier
    if (node.type === "Identifier") {
        return [node.name];
    }
    // name a static member access or a qualified type name by its parts
    else if (node.type === "MemberExpression" && !node.computed) {
        const head = qualifiedName(node.object);

        return head === undefined ? undefined : [...head, node.property.name];
    } else if (node.type === "TSQualifiedName") {
        const head = qualifiedName(node.left);

        return head === undefined ? undefined : [...head, node.right.name];
    }

    return undefined;
}

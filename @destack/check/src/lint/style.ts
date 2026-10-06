import type { ESTree } from "@oxlint/plugins";

/** The names StyleX's namespace is imported under. */
const NAMESPACES = new Set(["style", "stylex"]);

/** Report whether a call creates StyleX styles, as `style.create({...})` does. */
export function isCreateCall(node: ESTree.CallExpression): boolean {
    const callee = node.callee;

    return (
        callee.type === "MemberExpression" &&
        callee.object.type === "Identifier" &&
        NAMESPACES.has(callee.object.name) &&
        callee.property.type === "Identifier" &&
        callee.property.name === "create"
    );
}

/** Read a property's key as written, its text for an identifier or string and undefined for a computed key. */
export function keyOf(property: ESTree.ObjectProperty): string | undefined {
    if (property.computed) {
        return undefined;
    } else if (property.key.type === "Identifier") {
        return property.key.name;
    }

    return property.key.type === "Literal" && typeof property.key.value === "string"
        ? property.key.value
        : undefined;
}

/** Visit every property of a StyleX style object, through nested conditions and the bodies of dynamic styles. */
export function eachStyleProperty(
    node: ESTree.Node,
    visit: (property: ESTree.ObjectProperty) => void,
): void {
    // step into a dynamic style's returned object
    if (node.type === "ArrowFunctionExpression") {
        eachStyleProperty(node.body, visit);
        return;
    }
    if (node.type !== "ObjectExpression") {
        return;
    }

    // visit each property and the conditions nested in it
    for (const property of node.properties) {
        if (property.type === "Property") {
            visit(property);
            eachStyleProperty(property.value, visit);
        }
    }
}

/** Visit the properties of every style a `create` call defines. */
export function eachCreatedProperty(
    node: ESTree.CallExpression,
    visit: (property: ESTree.ObjectProperty) => void,
): void {
    const [styles] = node.arguments;
    if (styles?.type !== "ObjectExpression") {
        return;
    }
    for (const entry of styles.properties) {
        if (entry.type === "Property") {
            eachStyleProperty(entry.value, visit);
        }
    }
}

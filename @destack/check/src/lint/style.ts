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

/** Read a property's key as written. */
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

/** Visit every property of a StyleX style object. */
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

/** Visit every object of every style a `create` call defines, nested ones included. */
export function eachCreatedObject(
    node: ESTree.CallExpression,
    visit: (object: ESTree.ObjectExpression) => void,
): void {
    const [styles] = node.arguments;
    if (styles?.type === "ObjectExpression") {
        for (const entry of styles.properties) {
            if (entry.type === "Property") {
                eachObject(entry.value, visit);
            }
        }
    }
}

/** Visit a style's object and those nested in it, a dynamic style's returned one included. */
function eachObject(node: ESTree.Node, visit: (object: ESTree.ObjectExpression) => void): void {
    // step into a dynamic style's returned object
    if (node.type === "ArrowFunctionExpression") {
        eachObject(node.body, visit);
    } else if (node.type === "ObjectExpression") {
        visit(node);
        for (const property of node.properties) {
            if (property.type === "Property") {
                eachObject(property.value, visit);
            }
        }
    }
}

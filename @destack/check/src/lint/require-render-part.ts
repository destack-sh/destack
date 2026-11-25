import type { ESTree, Rule } from "@oxlint/plugins";

/** The package whose parts a caller draws as another element. */
const UI_PACKAGE = /[\\/]@destack[\\/]ui[\\/]/u;

/** The elements a part draws on its own, without the pictures and fields only they can be. */
const ELEMENTS: ReadonlySet<string> = new Set([
    "a",
    "article",
    "aside",
    "button",
    "dd",
    "div",
    "dl",
    "dt",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "label",
    "legend",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "section",
    "span",
    "ul",
]);

/** Draw a part by renderPart, which runs the caller's handlers and draws the caller's element. */
export const requireRenderPart: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "draw a part by renderPart" },
        messages: { part: "draw the part by renderPart in place of <{{name}}>" },
    },
    create(context) {
        // leave other packages to draw as they like
        if (!UI_PACKAGE.test(context.filename)) {
            return {};
        }

        return {
            JSXOpeningElement(node) {
                // find an element with a slot, the caller's styles and the caller's attributes
                const { name, attributes } = node;
                if (name.type !== "JSXIdentifier" || !ELEMENTS.has(name.name)) {
                    return;
                }
                const spreads = attributes.flatMap((attribute) =>
                    attribute.type === "JSXSpreadAttribute" ? [attribute.argument] : [],
                );
                const isPart =
                    attributes.some((attribute) => isSlot(attribute)) &&
                    spreads.some((spread) => isStyled(spread)) &&
                    spreads.some((spread) => !isStyle(spread));
                if (isPart) {
                    context.report({ node, messageId: "part", data: { name: name.name } });
                }
            },
        };
    },
};

/** Report whether an attribute names a part's slot. */
function isSlot(attribute: ESTree.JSXAttributeItem): boolean {
    return (
        attribute.type === "JSXAttribute" &&
        attribute.name.type === "JSXIdentifier" &&
        attribute.name.name === "data-slot"
    );
}

/** Report whether a spread is of styles. */
function isStyle(spread: ESTree.Expression): spread is ESTree.CallExpression {
    return (
        spread.type === "CallExpression" &&
        spread.callee.type === "MemberExpression" &&
        spread.callee.object.type === "Identifier" &&
        spread.callee.object.name === "style"
    );
}

/** Report whether a spread is of styles with a caller's `xstyle` in a list of them. */
function isStyled(spread: ESTree.Expression): boolean {
    // read the list the styles are given as
    const list = isStyle(spread) ? spread.arguments[0] : undefined;

    return (
        list?.type === "ArrayExpression" &&
        list.elements.some(
            (element) =>
                element?.type === "MemberExpression" &&
                element.property.type === "Identifier" &&
                element.property.name === "xstyle",
        )
    );
}

import type { ESTree, Rule } from "@oxlint/plugins";

/** Native constructors with a known message argument. */
const errorConstructors = new Set([
    "Error",
    "TypeError",
    "RangeError",
    "ReferenceError",
    "SyntaxError",
    "URIError",
    "EvalError",
    "AggregateError",
]);

/** The service failure constructor whose options carry the message. */
const SERVICE_ERROR = "ServiceError";

/** Check error messages without assuming custom constructor signatures. */
export const errorMessageStyle: Rule = {
    meta: {
        type: "suggestion",
        fixable: "code",
        docs: { description: "use explicit lowercase error messages without final periods" },
        schema: [],
        messages: {
            style: "[DF04] use a lowercase error message without a final period",
            missing: "[DF04] give the service error a message that says what was refused",
        },
    },
    create(context) {
        /** Report a literal message that starts uppercase or ends with a period. */
        function checkStyle(message: ESTree.Node | null | undefined): void {
            // preserve codes, acronyms, identifiers, and interpolated values
            const text =
                message?.type === "Literal" && typeof message.value === "string"
                    ? message.value
                    : message?.type === "TemplateLiteral" && message.expressions.length === 0
                      ? message.quasis[0].value.cooked
                      : undefined;
            if (message === null || message === undefined || text === undefined || text === null) {
                return;
            }

            // lowercase the first word and drop the final period
            if (/^[A-Z][a-z]+(?:\s|[.!?]|$)/.test(text) || /[^.]\.$/.test(text)) {
                const corrected = text
                    .replace(/^[A-Z](?=[a-z]+(?:\s|[.!?]|$))/, (letter) => letter.toLowerCase())
                    .replace(/(?<=[^.])\.$/, "");
                context.report({
                    node: message,
                    messageId: "style",
                    fix: (fixer) => fixer.replaceText(message, JSON.stringify(corrected)),
                });
            }
        }

        /** Check whether a name resolves to a local declaration. */
        function isShadowed(node: ESTree.Node, name: string): boolean {
            // walk the scopes outward
            let scope: ReturnType<typeof context.sourceCode.getScope> | null =
                context.sourceCode.getScope(node);
            while (scope) {
                const variable = scope.set.get(name);
                if (variable?.defs.length) {
                    return true;
                }
                scope = scope.upper;
            }

            return false;
        }

        return {
            ThrowStatement(node) {
                // inspect only native error constructor message positions
                const call = node.argument;
                if (call?.type !== "NewExpression" || call.callee.type !== "Identifier") {
                    return;
                }
                if (
                    !errorConstructors.has(call.callee.name) ||
                    isShadowed(node, call.callee.name)
                ) {
                    return;
                }

                // check the message argument
                checkStyle(call.arguments[call.callee.name === "AggregateError" ? 1 : 0]);
            },
            NewExpression(node) {
                // inspect only service errors
                if (node.callee.type !== "Identifier" || node.callee.name !== SERVICE_ERROR) {
                    return;
                }

                // find the message among literal options unless a spread may carry it
                const options = node.arguments[1];
                const properties = options?.type === "ObjectExpression" ? options.properties : [];
                const message = properties.find(
                    (property) =>
                        property.type === "Property" &&
                        !property.computed &&
                        property.key.type === "Identifier" &&
                        property.key.name === "message",
                );
                const isOpaque =
                    (options !== undefined && options.type !== "ObjectExpression") ||
                    properties.some((property) => property.type === "SpreadElement");

                // check a named message
                if (message?.type === "Property") {
                    checkStyle(message.value);
                }
                // require a message unless opaque options may carry it
                else if (!isOpaque) {
                    context.report({ node, messageId: "missing" });
                }
            },
        };
    },
};

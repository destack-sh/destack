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

/** A failure code, such as NOT_FOUND, that coded error constructors take before their message. */
const CODE = /^[A-Z][A-Z0-9_]*$/u;

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
        /** Report an interpolated message whose first text starts uppercase or whose last text ends with a period. */
        function checkTemplate(message: ESTree.TemplateLiteral): void {
            // read the text before the first and after the last interpolation
            const [first] = message.quasis;
            const last = message.quasis.at(-1);
            if (first === undefined || last === undefined) {
                throw new TypeError("a template literal has no text");
            }
            const isUpper = /^[A-Z][a-z]+(?:\s|[.!?:]|$)/u.test(first.value.raw);
            const isEnded = /(?:^|[^.])\.$/u.test(last.value.raw);
            if (!isUpper && !isEnded) {
                return;
            }

            // lowercase the first word and drop the final period in the source text
            const source = context.sourceCode.getText(message);
            const lowered = isUpper
                ? `\`${source.charAt(1).toLowerCase()}${source.slice(2)}`
                : source;
            const corrected = isEnded ? lowered.replace(/(?<=[^.])\.`$/u, "`") : lowered;
            context.report({
                node: message,
                messageId: "style",
                fix: (fixer) => fixer.replaceText(message, corrected),
            });
        }

        /** Report a literal message that starts uppercase or ends with a period. */
        function checkStyle(message: ESTree.Node | null | undefined): void {
            // check interpolated messages by their text parts
            if (message?.type === "TemplateLiteral" && message.expressions.length > 0) {
                checkTemplate(message);

                return;
            }

            // preserve codes, acronyms, identifiers, and interpolated values
            const text =
                message?.type === "Literal" && typeof message.value === "string"
                    ? message.value
                    : message?.type === "TemplateLiteral" && message.expressions.length === 0
                      ? message.quasis[0]?.value.cooked
                      : undefined;
            if (message === null || message === undefined || text === undefined || text === null) {
                return;
            }

            // lowercase the first word and drop the final period
            if (/^[A-Z][a-z]+(?:\s|[.!?]|$)/u.test(text) || /[^.]\.$/u.test(text)) {
                const corrected = text
                    .replace(/^[A-Z](?=[a-z]+(?:\s|[.!?]|$))/u, (letter) => letter.toLowerCase())
                    .replace(/(?<=[^.])\.$/u, "");
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
                if (variable !== undefined && variable.defs.length > 0) {
                    return true;
                }
                scope = scope.upper;
            }

            return false;
        }

        return {
            NewExpression(node) {
                // inspect named constructors only
                if (node.callee.type !== "Identifier") {
                    return;
                }
                const name = node.callee.name;

                // check a native error's message argument
                if (errorConstructors.has(name) && !isShadowed(node, name)) {
                    checkStyle(node.arguments[name === "AggregateError" ? 1 : 0]);

                    return;
                }
                // check a coded error's message after its code
                else if (name !== SERVICE_ERROR && name.endsWith("Error")) {
                    const code = node.arguments[0];
                    const isCoded =
                        code?.type === "Literal" &&
                        typeof code.value === "string" &&
                        CODE.test(code.value);
                    if (isCoded) {
                        checkStyle(node.arguments[1]);
                    }

                    return;
                }
                // leave every other constructor alone
                else if (name !== SERVICE_ERROR) {
                    return;
                }

                // find a service error's message among literal options unless a spread may carry it
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

import type { Rule } from "@oxlint/plugins";

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

/** Check directly thrown literal messages without assuming custom constructor signatures. */
export const errorMessageStyle: Rule = {
    meta: {
        type: "suggestion",
        fixable: "code",
        docs: { description: "use lowercase error messages without final periods" },
        schema: [],
        messages: { style: "[DF07] use a lowercase error message without a final period" },
    },
    create(context) {
        return {
            ThrowStatement(node) {
                // inspect only native error constructor message positions
                const call = node.argument;
                if (call?.type !== "NewExpression" || call.callee.type !== "Identifier") {
                    return;
                }
                if (!errorConstructors.has(call.callee.name)) {
                    return;
                }

                // ignore a locally shadowed native constructor
                let scope: ReturnType<typeof context.sourceCode.getScope> | null =
                    context.sourceCode.getScope(node);
                while (scope) {
                    const variable = scope.set.get(call.callee.name);
                    if (variable?.defs.length) {
                        return;
                    }
                    scope = scope.upper;
                }

                // preserve codes, acronyms, identifiers, and interpolated values
                const message = call.arguments[call.callee.name === "AggregateError" ? 1 : 0];
                const text =
                    message?.type === "Literal" && typeof message.value === "string"
                        ? message.value
                        : message?.type === "TemplateLiteral" && message.expressions.length === 0
                          ? message.quasis[0].value.cooked
                          : undefined;
                if (text === undefined || text === null) {
                    return;
                }
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
            },
        };
    },
};

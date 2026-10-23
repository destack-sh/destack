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

/** A failure reason, such as NOTE_NOT_FOUND, that an error family takes before its message. */
const CODE = /^[A-Z][A-Z0-9_]*$/u;

/** The call failure constructor whose options include the message. */
const CALL_ERROR = "CallError";

/** The family failure constructor that takes its message after the failure. */
const FAMILY_ERROR = "FamilyError";

/** The call that declares an error family. */
const DEFINE_ERROR_FAMILY = "defineErrorFamily";

/** Check error messages without assuming custom constructor signatures. */
export const errorMessageStyle: Rule = {
    meta: {
        type: "suggestion",
        fixable: "code",
        docs: { description: "use explicit lowercase error messages without final periods" },
        schema: [],
        messages: {
            style: "use a lowercase error message without a final period",
            missing: "give the call error a message that says what was refused",
        },
    },
    create(context) {
        /** Report an interpolated message with bad casing or punctuation. */
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
            // check interpolated messages by their text spans
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

        /** Find the declaration a name resolves to. */
        function resolve(node: ESTree.Node, name: string) {
            // walk the scopes outward
            let scope: ReturnType<typeof context.sourceCode.getScope> | null =
                context.sourceCode.getScope(node);
            while (scope) {
                const variable = scope.set.get(name);
                if (variable !== undefined && variable.defs.length > 0) {
                    return variable.defs[0];
                }
                scope = scope.upper;
            }

            return undefined;
        }

        /** Check whether a name resolves to a local declaration. */
        function isShadowed(node: ESTree.Node, name: string): boolean {
            return resolve(node, name) !== undefined;
        }

        /** Check whether a name is an error family, declared here or imported from its module. */
        function isFamily(node: ESTree.Node, name: string): boolean {
            // accept imported bindings, whose declaring module this file cannot see
            const definition = resolve(node, name);
            if (definition?.type === "ImportBinding") {
                return true;
            }

            // accept a local defineErrorFamily binding
            const declarator = definition?.node;
            const initializer = declarator?.type === "VariableDeclarator" ? declarator.init : null;

            return (
                initializer?.type === "CallExpression" &&
                initializer.callee.type === "Identifier" &&
                initializer.callee.name === DEFINE_ERROR_FAMILY
            );
        }

        /** Check a call error's message among literal options unless a spread may contain it. */
        function checkOptions(node: ESTree.Node, options: ESTree.Node | undefined): void {
            // find the message property
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
            // require a message unless opaque options may contain it
            else if (!isOpaque) {
                context.report({ node, messageId: "missing" });
            }
        }

        return {
            CallExpression(node) {
                // read a static member call, as in NoteError.of or CallError.http
                const { callee } = node;
                if (
                    callee.type !== "MemberExpression" ||
                    callee.computed ||
                    callee.object.type !== "Identifier" ||
                    callee.property.type !== "Identifier"
                ) {
                    return;
                }
                const owner = callee.object.name;
                const method = callee.property.name;

                // check a family failure's message after its reason, as in NoteError.of(reason, message)
                const reason = node.arguments[0];
                const isCoded =
                    reason?.type === "Literal" &&
                    typeof reason.value === "string" &&
                    CODE.test(reason.value);
                if (method === "of" && isCoded && isFamily(node, owner)) {
                    checkStyle(node.arguments[1]);
                }
                // check a call error's options, as in CallError.http(code, { message })
                else if (method === "http" && owner === CALL_ERROR) {
                    checkOptions(node, node.arguments[1]);
                }
            },
            NewExpression(node) {
                // inspect named constructors only
                if (node.callee.type !== "Identifier") {
                    return;
                }
                const name = node.callee.name;

                // check a native error's message argument
                if (errorConstructors.has(name) && !isShadowed(node, name)) {
                    checkStyle(node.arguments[name === "AggregateError" ? 1 : 0]);
                }
                // check a family failure's message after the failure
                else if (name === FAMILY_ERROR) {
                    checkStyle(node.arguments[1]);
                }
                // check a call error's options
                else if (name === CALL_ERROR) {
                    checkOptions(node, node.arguments[1]);
                }
            },
        };
    },
};

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

/** The names of counts and positions, and the camelCase endings that mark one. */
const COUNT_NAMES =
    /^(?:status|code|length|size|count|limit|line|column|timeout|lag|epoch|sequence|offset|start|end|position|port|attempts|steps|seconds|milliseconds|bits|written|completed|wait|capacity|index|floor|minimum|maximum)$|[a-z](?:Number|Count|Index|Size|Length)$/u;

/** The count names that also name text, such as an exit code or a region code, kept as written. */
const AMBIGUOUS_NAMES = /^(?:code|line|status|start|end|position)$/u;

/** The units after a count, as in `${size} bytes`. */
const COUNT_UNITS =
    /^ (?:ms|milliseconds|seconds|minutes|bytes|characters|rows|steps|turns|times|attempts|items|entries|redirects|terms|calls|bits|approvals|approvers)\b/u;

/** The names of kinds, which a message leaves bare, as Kubernetes' `pods "nginx" not found`. */
const KIND_NAMES = /^(?:kind|type|channel|storage)$/u;

/** The owners whose `.name` is a kind, as in `${object.name} '${id}' not found`. */
const KIND_OWNERS = /^(?:object|type)$/u;

/** The names of embedded text, such as another error's message. */
const TEXT_NAMES = /^(?:message|description|detail)$|[a-z](?:Message|Description|Detail)$/u;

/** Our own constants, in SCREAMING_CASE. */
const CONSTANT_NAMES = /^[A-Z][A-Z0-9_]*$/u;

/** The status phrase each condition's message carries, after Kubernetes' API error templates. */
const CONDITION_SHAPES: readonly (readonly [condition: string, shape: RegExp, words: string])[] = [
    ["NOT_FOUND", /\bnot found(?:$|: )/u, "<noun> '<value>' not found"],
    ["EXISTS", /\balready exists(?:$|: )/u, "<noun> '<value>' already exists"],
    ["CONFLICT", /^cannot \S.*: \S/u, "cannot <verb> <noun> '<value>': <why>"],
    ["INVALID", /\b(?:is|are) invalid: \S/u, "<noun> '<value>' is invalid: <what> must …"],
    ["REQUIRED", /\bis required(?:$|: )/u, "<what> is required"],
    ["UNAUTHENTICATED", /^authentication required(?:$|: )/u, "authentication required: <why>"],
    ["DENIED", /\bis forbidden: \S/u, "<noun> '<value>' is forbidden: <who> cannot <verb>"],
    ["BLOCKED", /^cannot \S.*: \S/u, "cannot <verb> <noun> '<value>': <why>"],
    ["EXHAUSTED", /\bexceeds\b/u, "<noun> '<value>' exceeds <limit>"],
    ["UNSUPPORTED", /\bis not supported(?:$|: )/u, "<thing> is not supported"],
    ["MOVED", /\bmoved to\b/u, "<noun> '<value>' moved to '<where>'"],
    ["UNAVAILABLE", /\bis unavailable(?:$|: )/u, "<noun> '<value>' is unavailable: <why>"],
    ["UNREACHABLE", /^could not\b/u, "could not <verb> <noun> '<value>'"],
    ["CANCELLED", /\bwas cancelled(?:$|: )/u, "<what> was cancelled"],
];

/** Words an error message never uses, as they hide what is wrong. */
const BANNED_WORDS = /\b(?:illegal|bad|unable|please|failed to)\b/iu;

/** Read the name an expression reads: an identifier, or a member's property. */
function nameOf(expression: ESTree.Node): string | undefined {
    if (expression.type === "Identifier") {
        return expression.name;
    } else if (
        expression.type === "MemberExpression" &&
        (expression.property.type === "Identifier" ||
            expression.property.type === "PrivateIdentifier")
    ) {
        return expression.property.name;
    }

    return undefined;
}

/** Report whether a call is `Owner.method(…)`, such as `JSON.stringify` or any `Math` call. */
function isStaticCall(expression: ESTree.Node, owner: string, method?: string): boolean {
    return (
        expression.type === "CallExpression" &&
        expression.callee.type === "MemberExpression" &&
        expression.callee.object.type === "Identifier" &&
        expression.callee.object.name === owner &&
        (method === undefined || nameOf(expression.callee) === method)
    );
}

/** Report whether a `.name` reads an object's or type's name, which is its kind. */
function isKindName(expression: ESTree.Node): boolean {
    const owner = expression.type === "MemberExpression" ? nameOf(expression.object) : undefined;

    return nameOf(expression) === "name" && owner !== undefined && KIND_OWNERS.test(owner);
}

/** Report whether a node is a number literal. */
function isNumber(node: ESTree.Node): boolean {
    return node.type === "Literal" && typeof node.value === "number";
}

/** Report whether a sum or other arithmetic is a count, a `+` only beside a number literal. */
function isArithmetic(expression: ESTree.Node): boolean {
    return (
        expression.type === "BinaryExpression" &&
        (expression.operator !== "+" || isNumber(expression.left) || isNumber(expression.right))
    );
}

/** Report whether an interpolation stays bare: a count, kind, text, JSON or dump, never a value. */
function isBare(expression: ESTree.Node, before: string, after: string): boolean {
    // read an optional chain by the expression it wraps, and a fallback by its first choice
    if (expression.type === "ChainExpression") {
        return isBare(expression.expression, before, after);
    } else if (expression.type === "LogicalExpression" && expression.operator === "??") {
        return isBare(expression.left, before, after);
    }

    // read the value by its shape, then by its name
    const name = nameOf(expression);

    return (
        isNumber(expression) ||
        isArithmetic(expression) ||
        isStaticCall(expression, "Math") ||
        isStaticCall(expression, "JSON", "stringify") ||
        (expression.type === "CallExpression" && nameOf(expression.callee) === "toFixed") ||
        before.endsWith("\n") ||
        after === " '" ||
        COUNT_UNITS.test(after) ||
        isKindName(expression) ||
        (name !== undefined &&
            (COUNT_NAMES.test(name) ||
                KIND_NAMES.test(name) ||
                TEXT_NAMES.test(name) ||
                CONSTANT_NAMES.test(name)))
    );
}

/** Check error messages without assuming custom constructor signatures. */
export const errorMessageStyle: Rule = {
    meta: {
        type: "suggestion",
        fixable: "code",
        docs: { description: "use explicit lowercase error messages without final periods" },
        schema: [],
        messages: {
            style: "use a lowercase error message without a final period",
            quote: "quote each interpolated value in single quotes, as in installation '${name}' not found",
            unquote: "leave counts, kinds and embedded text bare, as in exceeds ${limit} rows",
            shape: "write a {{condition}} message as '{{words}}'",
            word: "say what is wrong instead of '{{word}}'",
            missing: "give the call error a message that says what was refused",
        },
    },
    create(context) {
        /** Report an interpolated message with bad casing, punctuation or an unquoted value. */
        function checkTemplate(message: ESTree.TemplateLiteral): void {
            // read the text before the first and after the last interpolation
            const { quasis, expressions } = message;
            const [first] = quasis;
            const last = quasis.at(-1);
            if (first === undefined || last === undefined) {
                throw new TypeError("a template literal has no text");
            }
            const isUpper = /^[A-Z][a-z]+(?:\s|[.!?:]|$)/u.test(first.value.raw);
            const isEnded = /(?:^|[^.])\.$/u.test(last.value.raw);

            // group interpolations joined without whitespace into one value
            const grouped: { readonly start: number; readonly end: number }[] = [];
            for (const index of expressions.keys()) {
                const previous = grouped.at(-1);
                const joined = quasis[index]?.value.cooked ?? "";
                if (previous?.end === index - 1 && !/\s/u.test(joined)) {
                    grouped[grouped.length - 1] = { start: previous.start, end: index };
                } else {
                    grouped.push({ start: index, end: index });
                }
            }

            // quote every value but a lone bare interpolation
            const runs = grouped.filter(({ start, end }) => {
                // quote the value unless every part of it is bare
                return !expressions
                    .slice(start, end + 1)
                    .every((part, offset) =>
                        isBare(
                            part,
                            quasis[start + offset]?.value.cooked ?? "",
                            quasis[start + offset + 1]?.value.cooked ?? "",
                        ),
                    );
            });
            const isUnquoted = runs.some(
                ({ start, end }) =>
                    !/'[^\s']*$/u.test(quasis[start]?.value.raw ?? "") ||
                    !/^[^\s']*'/u.test(quasis[end + 1]?.value.raw ?? ""),
            );

            // unquote a lone bare interpolation, unless its name or a constant may be text
            const unquoted = new Set(
                grouped
                    .filter(({ start, end }) => {
                        // read the value, the text around it and its name
                        const part = expressions[start];
                        const before = quasis[start]?.value.cooked ?? "";
                        const after = quasis[end + 1]?.value.cooked ?? "";
                        const name = part === undefined ? undefined : nameOf(part);

                        return (
                            start === end &&
                            part !== undefined &&
                            before.endsWith("'") &&
                            after.startsWith("'") &&
                            isBare(part, before.slice(0, -1), after) &&
                            !(
                                name !== undefined &&
                                (AMBIGUOUS_NAMES.test(name) || CONSTANT_NAMES.test(name))
                            )
                        );
                    })
                    .map(({ start }) => start),
            );
            const isQuoted = unquoted.size > 0;
            if (!isUpper && !isEnded && !isUnquoted && !isQuoted) {
                return;
            }

            // rebuild the text: lowercase the first word, quote each value, drop the final period
            const opens = new Set(runs.map((run) => run.start));
            const closes = new Set(runs.map((run) => run.end + 1));
            const texts = quasis.map((quasi, index) => {
                // drop the quotes of a bare value, open a value before its run, close it after
                const stripped = unquoted.has(index)
                    ? quasi.value.raw.slice(0, -1)
                    : quasi.value.raw;
                const raw = unquoted.has(index - 1) ? stripped.slice(1) : stripped;
                const opened = opens.has(index) && !raw.endsWith("'") ? `${raw}'` : raw;
                const closed = closes.has(index) && !raw.startsWith("'") ? `'${opened}` : opened;
                const lowered =
                    index === 0 && isUpper
                        ? `${closed.charAt(0).toLowerCase()}${closed.slice(1)}`
                        : closed;

                return index === quasis.length - 1 && isEnded
                    ? lowered.replace(/(?<=[^.])\.$/u, "")
                    : lowered;
            });
            const corrected = texts
                .map((text, index) => {
                    const expression = expressions[index];

                    return expression === undefined
                        ? text
                        : `${text}\${${context.sourceCode.getText(expression)}}`;
                })
                .join("");
            context.report({
                node: message,
                messageId: isUnquoted ? "quote" : isQuoted ? "unquote" : "style",
                fix: (fixer) => fixer.replaceText(message, `\`${corrected}\``),
            });
        }

        /** Report a message that uses a banned word or lacks its reason condition's shape. */
        function checkShape(message: ESTree.Node | null | undefined, reason?: string): void {
            // read the message's text, each interpolated value as a placeholder
            const text =
                message?.type === "Literal" && typeof message.value === "string"
                    ? message.value
                    : message?.type === "TemplateLiteral"
                      ? message.quasis.map((quasi) => quasi.value.raw).join("…")
                      : undefined;
            if (message === null || message === undefined || text === undefined) {
                return;
            }

            // refuse a banned word, then require the condition's words
            const banned = BANNED_WORDS.exec(text)?.[0];
            const shape =
                reason === undefined
                    ? undefined
                    : CONDITION_SHAPES.find(([condition]) => reason.endsWith(`_${condition}`));
            if (banned !== undefined) {
                context.report({ node: message, messageId: "word", data: { word: banned } });
            } else if (shape !== undefined && !shape[1].test(text)) {
                context.report({
                    node: message,
                    messageId: "shape",
                    data: { condition: shape[0], words: shape[2] },
                });
            }
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
                    checkShape(node.arguments[1], reason.value);
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
                    const message = node.arguments[name === "AggregateError" ? 1 : 0];
                    checkStyle(message);
                    checkShape(message);
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

import type { ESTree, Rule } from "@oxlint/plugins";
import { isTestFile } from "./word.ts";

/** The test and hook functions, whose last argument or options may carry a timeout. */
const TIMED = new Set([
    "test",
    "it",
    "describe",
    "beforeAll",
    "beforeEach",
    "afterAll",
    "afterEach",
]);

/** The configuration keys setting a test tier's timeouts and slow threshold. */
const TIER_KEYS = new Set(["testTimeout", "hookTimeout", "slowTestThreshold"]);

/** Require tests to take their timeouts from their tier alone. */
export const noTestTimeout: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "take test timeouts from the test tier alone" },
        messages: {
            timeout: "remove the timeout: tests take their hang ceiling from their tier",
            key: "remove '{{key}}': the test tier sets it",
        },
    },
    create(context) {
        // check test modules and test configurations only
        if (!isTestFile(context.filename)) {
            return {};
        }
        const isConfiguration = /(?:^|[\\/])vitest\.config\.[cm]?[jt]s$/u.test(context.filename);

        return {
            CallExpression(node) {
                // report a test's or hook's timeout, as options or as a trailing number
                const name = calleeName(node.callee);
                const { arguments: values } = node;
                const last = values.at(-1);
                const hasFunction = values.some(isFunction);
                if (name === "poll") {
                    if (values.some(hasTimeout)) {
                        context.report({ node, messageId: "timeout" });
                    }
                } else if (name !== undefined && TIMED.has(name)) {
                    const isTrailing = hasFunction && last !== undefined && !isFunction(last);
                    if (isTrailing || values.some(hasTimeout)) {
                        context.report({ node, messageId: "timeout" });
                    }
                }
            },
            Property(node) {
                // report a tier key set by hand in a test configuration
                const key = node.key.type === "Identifier" ? node.key.name : undefined;
                if (isConfiguration && key !== undefined && TIER_KEYS.has(key)) {
                    context.report({ node, messageId: "key", data: { key } });
                }
            },
        };
    },
};

/** Read a call's function name: `test` for `test(…)`, `test.skip(…)` and `test.each(…)(…)`. */
function calleeName(callee: ESTree.Node): string | undefined {
    // name a plain call
    if (callee.type === "Identifier") {
        return callee.name;
    }
    // name a member call by its object, or `poll` for `expect.poll`
    else if (callee.type === "MemberExpression") {
        const isPoll = callee.property.type === "Identifier" && callee.property.name === "poll";

        return isPoll ? "poll" : calleeName(callee.object);
    }
    // name a curried call by its inner call
    else if (callee.type === "CallExpression") {
        return calleeName(callee.callee);
    }
    // name no other callee
    else {
        return undefined;
    }
}

/** Report whether an argument is a function. */
function isFunction(node: ESTree.Node): boolean {
    return node.type === "ArrowFunctionExpression" || node.type === "FunctionExpression";
}

/** Report whether an argument is options with a timeout. */
function hasTimeout(node: ESTree.Node): boolean {
    return (
        node.type === "ObjectExpression" &&
        node.properties.some(
            (property) =>
                property.type === "Property" &&
                property.key.type === "Identifier" &&
                property.key.name === "timeout",
        )
    );
}

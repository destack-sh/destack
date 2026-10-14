import type { Rule } from "@oxlint/plugins";

/** The calls that read the clock, draw at random or use the network, by object and member. */
const NONDETERMINISTIC: Readonly<Record<string, readonly string[]>> = {
    "": ["fetch"],
    Date: ["now"],
    Math: ["random"],
    crypto: ["randomUUID", "getRandomValues"],
    performance: ["now"],
};

/** The phases of a handler that run in the transaction, again on every attempt. */
const TRANSACTION_PHASES = new Set(["handle", "authorize"]);

/** A syntax node the rule walks, by the fields it reads. */
interface Node {
    /** The node's syntax type. */
    readonly type: string;
    /** The node containing this one, absent at the root. */
    readonly parent?: Node | null;
}

/** Reject reading the clock, drawing at random and using the network in method handlers. */
export const deterministicHandler: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "keep method handlers deterministic" },
        messages: {
            call: "a handler runs again on every attempt: use {{replacement}} in place of {{name}}",
        },
    },
    create(context) {
        return {
            CallExpression(node) {
                // read the called global or member
                const callee = node.callee;
                const name =
                    callee.type === "Identifier"
                        ? { object: "", member: callee.name }
                        : callee.type === "MemberExpression" &&
                            callee.object.type === "Identifier" &&
                            callee.property.type === "Identifier"
                          ? { object: callee.object.name, member: callee.property.name }
                          : undefined;

                // report a nondeterministic call inside a handler
                const isNondeterministic =
                    name !== undefined &&
                    (NONDETERMINISTIC[name.object]?.includes(name.member) ?? false);
                if (name !== undefined && isNondeterministic && isInHandler(node)) {
                    const called =
                        name.object === "" ? name.member : `${name.object}.${name.member}`;
                    context.report({
                        node,
                        messageId: "call",
                        data: { name: `${called}()`, replacement: replacementOf(called) },
                    });
                }
            },
            NewExpression(node) {
                // report reading the clock through a date of no arguments inside a handler
                const isClock =
                    node.callee.type === "Identifier" &&
                    node.callee.name === "Date" &&
                    node.arguments.length === 0;
                if (isClock && isInHandler(node)) {
                    context.report({
                        node,
                        messageId: "call",
                        data: { name: "new Date()", replacement: "call.now" },
                    });
                }
            },
        };
    },
};

/** Read the call method a handler uses in place of a nondeterministic call. */
function replacementOf(called: string): string {
    if (called === "Math.random" || called === "crypto.getRandomValues") {
        return "call.random()";
    } else if (called === "crypto.randomUUID") {
        return "call.generateId(type)";
    } else if (called === "fetch") {
        return "a prepare phase";
    } else {
        return "call.now";
    }
}

/** Decide whether a node sits in a method's handling or authorization. */
function isInHandler(node: Node): boolean {
    for (let current = node.parent; current !== undefined && current !== null;) {
        const parent = current.parent;
        if (
            isFunction(current) &&
            parent !== undefined &&
            parent !== null &&
            isHandler(current, parent)
        ) {
            return true;
        }
        current = parent;
    }

    return false;
}

/** Decide whether a function handles or authorizes a method in `.implement({ methods })`. */
function isHandler(function_: Node, property: Node): boolean {
    // read the object the function's property sits in
    if (property.type !== "Property" || !("value" in property) || property.value !== function_) {
        return false;
    }
    const object = property.parent;
    if (object === undefined || object === null || object.type !== "ObjectExpression") {
        return false;
    }

    // accept a method's value in the implemented methods, or a transaction phase of its handler
    const key = "key" in property && isIdentifier(property.key) ? property.key.name : undefined;
    const handler = object.parent;
    const isTransactionPhase =
        key !== undefined &&
        TRANSACTION_PHASES.has(key) &&
        handler !== undefined &&
        handler !== null &&
        handler.type === "Property" &&
        isImplemented(handler.parent);

    return isImplemented(object) || isTransactionPhase;
}

/** Decide whether an object is the `methods` of an `.implement({...})` call's argument. */
function isImplemented(object: Node | null | undefined): boolean {
    // read the methods property and the implementation with it
    const property = object?.parent;
    const implementation = property?.parent;
    const call = implementation?.parent;
    const isMethods =
        property !== undefined &&
        property !== null &&
        property.type === "Property" &&
        "key" in property &&
        isIdentifier(property.key) &&
        property.key.name === "methods";

    return (
        isMethods &&
        call !== undefined &&
        call !== null &&
        call.type === "CallExpression" &&
        "callee" in call &&
        isMember(call.callee, "implement")
    );
}

/** Decide whether a node is a function. */
function isFunction(node: Node): boolean {
    return node.type === "ArrowFunctionExpression" || node.type === "FunctionExpression";
}

/** Decide whether a node is an identifier. */
function isIdentifier(
    node: unknown,
): node is { readonly type: "Identifier"; readonly name: string } {
    return (
        typeof node === "object" && node !== null && "type" in node && node.type === "Identifier"
    );
}

/** Decide whether a node reads a member by name. */
function isMember(node: unknown, member: string): boolean {
    return (
        typeof node === "object" &&
        node !== null &&
        "type" in node &&
        node.type === "MemberExpression" &&
        "property" in node &&
        isIdentifier(node.property) &&
        node.property.name === member
    );
}

import MagicString from "magic-string";
import { type ESTree, parseSync } from "rolldown/utils";
import type { Plugin } from "vite";

/** The module-level binding opening a frame in the realm's context. */
const CONTEXT_BINDING = "__destackContext";

/** The binding of an async function call's frame. */
const FRAME_BINDING = "__destackFrame";

/** The statement defining the binding, reading the realm's context at each call. */
const CONTEXT_PRELUDE = `const ${CONTEXT_BINDING} = { enter: () => globalThis[globalThis.Symbol.for("destack.context")].enter() };\n`;

/** The modules a browser build compiles, by their path's extension. */
const MODULE_PATTERN = /\.(?:[cm]?[jt]sx?)(?:\?.*)?$/u;

/** Carry the telemetry context across awaits in a browser build's modules. */
export function contextPlugin(): Plugin {
    return {
        name: "destack-context",
        enforce: "post",
        transform(code, id) {
            // frame each async function of a script module
            if (!MODULE_PATTERN.test(id) || !code.includes("async")) {
                return undefined;
            }
            const framed = frameModule(code, id);
            if (framed === undefined) {
                return undefined;
            }

            return {
                code: framed.toString(),
                map: framed.generateMap({ hires: true, source: id }),
            };
        },
    };
}

/** Open a frame in each async function of a module, or nothing when it declares none. */
export function frameModule(code: string, id: string): MagicString | undefined {
    // parse the module, refusing one that does not parse
    const parsed = parseSync(id, code, { lang: "js", sourceType: "module" });
    if (parsed.errors.length > 0) {
        throw new SyntaxError(`module '${id}' does not parse for its context frames`);
    }

    // frame each async function, reading the realm's context once at the module's top
    const edited = new MagicString(code);
    const framed = frameNode(parsed.program, undefined, undefined, edited, code);
    if (!framed) {
        return undefined;
    }
    edited.prepend(CONTEXT_PRELUDE);

    return edited;
}

/** Frame a node and its children, reporting whether any async function opened a frame. */
function frameNode(
    node: ESTree.Node,
    parent: ESTree.Node | undefined,
    frame: ESTree.Function | ESTree.ArrowFunctionExpression | undefined,
    edited: MagicString,
    code: string,
): boolean {
    // enter a function: an async one opens its own frame, any other leaves none to its body
    let inner = frame;
    let isFramed = false;
    if (
        node.type === "FunctionDeclaration" ||
        node.type === "FunctionExpression" ||
        node.type === "ArrowFunctionExpression"
    ) {
        inner = node.async ? node : undefined;
        if (node.async && node.body !== null && node.body !== undefined) {
            frameBody(node, edited, code);
            isFramed = true;
        }
    }
    // suspend and resume around an await or a yield of the framing function
    else if (frame !== undefined && node.type === "AwaitExpression") {
        edited.appendRight(node.start, `${FRAME_BINDING}.resume(`);
        edited.appendRight(node.argument.start, `${FRAME_BINDING}.suspend(`);
        edited.appendLeft(node.argument.end, ")");
        edited.appendLeft(node.end, ")");
    } else if (frame !== undefined && node.type === "YieldExpression" && frame.async) {
        const { argument } = node;
        edited.appendRight(node.start, `${FRAME_BINDING}.resume(`);
        if (argument === null) {
            edited.appendLeft(node.end, ` ${FRAME_BINDING}.suspend(undefined)`);
        } else {
            edited.appendRight(argument.start, `${FRAME_BINDING}.suspend(`);
            edited.appendLeft(argument.end, ")");
        }
        edited.appendLeft(node.end, ")");
    }
    // suspend before each step of a for await loop, resuming in its body and after it, its label kept on it
    else if (frame !== undefined && node.type === "ForOfStatement" && node.await) {
        edited.appendRight(node.right.start, `${FRAME_BINDING}.each(`);
        edited.appendLeft(node.right.end, ")");
        resumeIn(node.body, edited);
        resumeAfter(parent?.type === "LabeledStatement" ? parent : node, edited);
    }
    // resume after a block disposing a declaration at its end, which awaits the disposal, a function's body aside as its frame exits there
    else if (
        frame !== undefined &&
        node.type === "BlockStatement" &&
        frame.body !== node &&
        isDisposing(node)
    ) {
        resumeAfter(node, edited);
    }
    // resume in a catch or finally block, which a rejected await enters
    else if (frame !== undefined && node.type === "TryStatement") {
        if (node.handler !== null) {
            resumeIn(node.handler.body, edited);
        }
        if (node.finalizer !== null) {
            resumeIn(node.finalizer, edited);
        }
    }

    // frame the children
    for (const child of children(node)) {
        isFramed = frameNode(child, node, inner, edited, code) || isFramed;
    }

    return isFramed;
}

/** Open an async function's frame at its body's start and exit it at its end. */
function frameBody(
    declared: ESTree.Function | ESTree.ArrowFunctionExpression,
    edited: MagicString,
    code: string,
): void {
    // wrap a block body in the frame
    const { body } = declared;
    const entered = `const ${FRAME_BINDING} = ${CONTEXT_BINDING}.enter(); try {`;
    const exited = `} finally { ${FRAME_BINDING}.exit(); }`;
    if (body !== null && body.type === "BlockStatement") {
        edited.appendLeft(body.start + 1, ` ${entered}`);
        edited.prependRight(body.end - 1, `${exited} `);
    }
    // turn an expression body into a block returning it, parentheses included
    else if (body !== null) {
        const arrow = code.lastIndexOf("=>", body.start);
        edited.appendLeft(arrow + 2, ` { ${entered} return (`);
        edited.prependRight(declared.end, `); ${exited} }`);
    }
}

/** Resume the frame at a statement's start, wrapping a statement that is no block. */
function resumeIn(statement: ESTree.Node, edited: MagicString): void {
    // resume first in a block
    if (statement.type === "BlockStatement") {
        edited.appendLeft(statement.start + 1, ` ${FRAME_BINDING}.resume();`);
    }
    // wrap any other statement in a block resuming first
    else {
        edited.appendRight(statement.start, `{ ${FRAME_BINDING}.resume(); `);
        edited.appendLeft(statement.end, " }");
    }
}

/** Resume the frame after a statement, wrapping both in a block. */
function resumeAfter(statement: ESTree.Node, edited: MagicString): void {
    edited.appendRight(statement.start, "{ ");
    edited.appendLeft(statement.end, ` ${FRAME_BINDING}.resume(); }`);
}

/** Read whether a block declares an await using declaration of its own. */
function isDisposing(block: ESTree.BlockStatement): boolean {
    return block.body.some(
        (statement) => statement.type === "VariableDeclaration" && statement.kind === "await using",
    );
}

/** List a node's child nodes in source order. */
function children(node: ESTree.Node): ESTree.Node[] {
    // take each property holding a node or a list of nodes
    const found: ESTree.Node[] = [];
    for (const [key, value] of Object.entries(node)) {
        if (key === "parent") {
            continue;
        }
        if (Array.isArray(value)) {
            for (const item of value) {
                if (isNode(item)) {
                    found.push(item);
                }
            }
        } else if (isNode(value)) {
            found.push(value);
        }
    }

    return found;
}

/** Read whether a value is a syntax node. */
function isNode(value: unknown): value is ESTree.Node {
    return typeof value === "object" && value !== null && "type" in value && "start" in value;
}

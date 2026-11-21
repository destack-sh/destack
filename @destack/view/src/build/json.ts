import { parseAst } from "rolldown/parseAst";
import type { Plugin } from "@destack/package/build";

/** Drop the attributes of dynamic JSON imports, which Vite serves as script modules. */
export function jsonPlugin(): Plugin {
    return {
        name: "@destack/view/json",
        transform(code) {
            // leave modules without a dynamic import alone
            if (!code.includes("import(")) {
                return null;
            }

            // find each dynamic import of a JSON file that passes options
            const cuts: (readonly [number, number])[] = [];
            visit(parseAst(code), (node) => {
                if (
                    node.type !== "ImportExpression" ||
                    !("source" in node) ||
                    !("options" in node)
                ) {
                    return;
                }
                const { source, options } = node;
                if (isJsonLiteral(source) && isNode(options)) {
                    cuts.push([source.end, options.end]);
                }
            });
            if (cuts.length === 0) {
                return null;
            }

            // remove the options from the last import to the first, so earlier offsets hold
            const stripped = cuts
                .toSorted(([first], [second]) => second - first)
                .reduce((text, [start, end]) => text.slice(0, start) + text.slice(end), code);

            return { code: stripped, map: null };
        },
    };
}

/** A node of an ESTree syntax tree, with its type and the offsets it spans. */
interface SyntaxNode {
    /** The kind of node, such as `ImportExpression`. */
    readonly type: string;
    /** The offset of its first character. */
    readonly start: number;
    /** The offset after its last character. */
    readonly end: number;
}

/** Report whether a value is a syntax node. */
function isNode(value: unknown): value is SyntaxNode {
    return typeof value === "object" && value !== null && "type" in value && "end" in value;
}

/** Report whether a node is a string literal of a JSON file's path. */
function isJsonLiteral(value: unknown): value is SyntaxNode {
    return (
        isNode(value) &&
        value.type === "Literal" &&
        "value" in value &&
        typeof value.value === "string" &&
        value.value.endsWith(".json")
    );
}

/** Call a function on a node and every node under it. */
function visit(node: unknown, call: (node: SyntaxNode) => void): void {
    // visit the nodes of a list in turn
    if (Array.isArray(node)) {
        for (const child of node) {
            visit(child, call);
        }
    }
    // visit a node, then each node among its fields
    else if (isNode(node)) {
        call(node);
        for (const child of Object.values(node)) {
            visit(child, call);
        }
    }
}

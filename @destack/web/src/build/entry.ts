import { RolldownMagicString } from "rolldown";
import { type ESTree, parseSync, Visitor } from "rolldown/utils";
import type { Plugin } from "@destack/package/build";

/** The module Solid generates to handle page requests. */
const HANDLER = "virtual:solid-ssr-handler";

/** The server entry Solid generates, whose handler loads the client entry itself. */
const GENERATED_ENTRY = "virtual:solid-ssr-entry-server.tsx";

/** The flag the handler sets once it injects its head elements. */
const INJECTED = "injected";

/** The statement loading the client entry in the head, as Solid writes it for a generated entry. */
const CLIENT_ENTRY_SCRIPT =
    "chunk = chunk.replace('</head>', (clientEntry ? '<script type=\"module\"' + nonceAttr + ' src=\"' + clientEntry + '\" async></' + 'script>' : '') + '</head>');\n";

/** Load an authored client entry in rendered pages, as Solid loads a generated one. */
export function entryPlugin(): Plugin {
    return {
        name: "@destack/web/entry",
        transform: {
            filter: { id: new RegExp(`^${RegExp.escape(HANDLER)}$`, "u") },
            handler(code) {
                // parse the handler
                const parsed = parseSync(HANDLER, code);
                if (parsed.errors.length) {
                    throw new TypeError(`invalid generated module: ${HANDLER}`);
                }

                // leave the handler of generated entries, which loads the client entry itself
                const { isGenerated, injections } = readHandler(parsed.program);
                if (isGenerated) {
                    return undefined;
                }

                // require the one head injection to extend
                const [injection, ...others] = injections;
                if (injection === undefined || others.length > 0) {
                    throw new TypeError(`unknown head injection in ${HANDLER}`);
                }

                // load the client entry after the rest of the head
                const source = new RolldownMagicString(code);
                source.appendLeft(injection.end - 1, CLIENT_ENTRY_SCRIPT);

                return {
                    code: source.toString(),
                    map: source.generateMap({ source: HANDLER, hires: true }).toString(),
                };
            },
        },
    };
}

/** Read whether a handler imports the generated server entry, and the blocks injecting its head. */
function readHandler(program: ESTree.Program): {
    readonly isGenerated: boolean;
    readonly injections: readonly ESTree.BlockStatement[];
} {
    // collect the server entry import and the head injections
    let isGenerated = false;
    const injections: ESTree.BlockStatement[] = [];
    new Visitor({
        ImportDeclaration(node) {
            isGenerated ||= node.source.value === GENERATED_ENTRY;
        },
        IfStatement(node) {
            // select the block running once the handler has not yet injected its head
            const { test, consequent } = node;
            if (
                test.type === "LogicalExpression" &&
                test.left.type === "UnaryExpression" &&
                test.left.operator === "!" &&
                test.left.argument.type === "Identifier" &&
                test.left.argument.name === INJECTED &&
                consequent.type === "BlockStatement"
            ) {
                injections.push(consequent);
            }
        },
    }).visit(program);

    return { isGenerated, injections };
}

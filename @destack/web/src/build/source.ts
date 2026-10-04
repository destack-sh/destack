import { isAbsolute, relative, resolve, sep } from "node:path";
import { parseSync, transformSync, Visitor, type ESTree } from "rolldown/utils";
import type { Plugin } from "@destack/package/build";

/** Package-relative imports inserted into generated framework modules. */
const PREFIX = "virtual:@destack/web/source/";

/** A virtual entry name Vite writes relative to the working directory. */
const VIRTUAL_ENTRY = /(^|\/)virtual:solid-/u;

/** Give generated framework source stable imports and matching source maps. */
export function sourcePlugin(directory: string): Plugin {
    return {
        name: "@destack/web/source",
        enforce: "pre",
        resolveId: {
            filter: { id: new RegExp(`^${RegExp.escape(PREFIX)}`, "u") },
            async handler(source) {
                return await this.resolve(
                    resolve(directory, source.slice(PREFIX.length)),
                    undefined,
                    {
                        skipSelf: true,
                    },
                );
            },
        },
        transform: {
            filter: { id: /^\0?virtual:/u },
            handler(code, id) {
                // parse the generated module
                const parsed = parseSync(id, code);
                if (parsed.errors.length) {
                    throw new TypeError(`invalid generated module: ${id}`);
                }

                // replace only parsed module specifiers within the application directory
                const isManifest = id.replace(/^\0/u, "") === "virtual:solid-manifest";
                const specifiers = collectSpecifiers(parsed.program, isManifest);
                const rewritten = rewriteSpecifiers(code, specifiers, directory, isManifest);
                if (rewritten === undefined) {
                    return;
                }

                // start maps at the normalized generated source, preserving authored source maps
                const source = `virtual:@destack/web/${id.slice(id.indexOf("virtual:") + 8)}`;
                const result = transformSync(source, rewritten, {
                    jsx: "preserve",
                    sourcemap: true,
                    target: "esnext",
                });
                if (result.errors.length) {
                    throw new TypeError(`cannot transform generated module: ${id}`);
                }

                // require the source map the transform was asked for
                if (result.map === undefined) {
                    throw new TypeError(`missing source map for generated module: ${id}`);
                }

                return { code: result.code, map: result.map };
            },
        },
    };
}

/** Collect a module's literal specifiers by offset, and in the runtime manifest its virtual entry names. */
function collectSpecifiers(
    program: ESTree.Program,
    isManifest: boolean,
): Map<number, ESTree.StringLiteral> {
    const specifiers = new Map<number, ESTree.StringLiteral>();
    new Visitor({
        Literal(node) {
            // normalize Vite's cwd-relative virtual entry names in the runtime manifest
            if (isManifest && typeof node.value === "string" && VIRTUAL_ENTRY.test(node.value)) {
                specifiers.set(node.start, node);
            }
        },
        ImportDeclaration(node) {
            specifiers.set(node.source.start, node.source);
        },
        ExportNamedDeclaration(node) {
            if (node.source) {
                specifiers.set(node.source.start, node.source);
            }
        },
        ExportAllDeclaration(node) {
            specifiers.set(node.source.start, node.source);
        },
        ImportExpression(node) {
            if (node.source.type === "Literal" && typeof node.source.value === "string") {
                specifiers.set(node.source.start, node.source);
            }
        },
    }).visit(program);

    return specifiers;
}

/** Rewrite virtual entry names and absolute imports within the directory, absent when none applies. */
function rewriteSpecifiers(
    code: string,
    specifiers: ReadonlyMap<number, ESTree.StringLiteral>,
    directory: string,
    isManifest: boolean,
): string | undefined {
    // edit from the end to keep parser offsets valid
    let rewritten: string | undefined;
    const ordered = [...specifiers.values()].toSorted((left, right) => right.start - left.start);
    for (const specifier of ordered) {
        const replacement = replaceSpecifier(specifier.value, directory, isManifest);
        if (replacement !== undefined) {
            const text = JSON.stringify(replacement);
            const current = rewritten ?? code;
            rewritten = current.slice(0, specifier.start) + text + current.slice(specifier.end);
        }
    }

    return rewritten;
}

/** Name a specifier stably: a virtual entry by its name, an absolute import within the directory below the prefix. */
function replaceSpecifier(
    value: string,
    directory: string,
    isManifest: boolean,
): string | undefined {
    // name a manifest's virtual entry from its virtual prefix
    if (isManifest && VIRTUAL_ENTRY.test(value)) {
        return value.slice(value.indexOf("virtual:"));
    }
    // leave relative and bare specifiers
    else if (!isAbsolute(value)) {
        return undefined;
    }

    // name a module within the directory below the prefix
    const path = relative(directory, value).split(sep).join("/");
    const isOutside = path === ".." || path.startsWith("../") || isAbsolute(path);

    return isOutside ? undefined : PREFIX + path;
}

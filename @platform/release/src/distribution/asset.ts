import { realpath, stat } from "node:fs/promises";
import { isAbsolute, relative, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parseSync, Visitor, type ESTree } from "rolldown/utils";
import MagicString from "magic-string";

/** Preserve authored directory URLs while collecting executable assets. */
export async function transformAssets(
    source: string,
    path: string,
    root: string,
    assets: Set<string>,
): Promise<string | undefined> {
    // leave dependency loaders and modules without directory URLs unchanged
    if (path.includes(`${sep}node_modules${sep}`) || !source.includes("import.meta.url")) {
        return;
    }
    const parsed = parseSync(path, source);
    if (parsed.errors.length) {
        throw new Error(`invalid executable source: ${path}`);
    }

    // collect literal directory references before rewriting source positions
    const references: ESTree.NewExpression[] = [];
    new Visitor({
        NewExpression(node) {
            const [location, base] = node.arguments;
            if (
                node.callee.type === "Identifier" &&
                node.callee.name === "URL" &&
                node.arguments.length === 2 &&
                location?.type === "Literal" &&
                typeof location.value === "string" &&
                /^\.{1,2}\//.test(location.value) &&
                location.value.endsWith("/") &&
                base?.type === "MemberExpression" &&
                !base.computed &&
                base.property.type === "Identifier" &&
                base.property.name === "url" &&
                base.object.type === "MetaProperty" &&
                base.object.meta.name === "import" &&
                base.object.property.name === "meta"
            ) {
                references.push(node);
            }
        },
    }).visit(parsed.program);
    if (!references.length) {
        return;
    }

    // retain paths under the executable root without flattening separate package assets
    const output = new MagicString(source);
    for (const reference of references) {
        const location = reference.arguments[0] as ESTree.StringLiteral;
        const url = new URL(location.value, pathToFileURL(path));
        const directory = await realpath(fileURLToPath(url));
        const local = relative(root, directory);
        if (local === ".." || local.startsWith(`..${sep}`) || isAbsolute(local)) {
            throw new Error(`executable asset leaves its root: ${directory}`);
        }
        if (!(await stat(directory)).isDirectory()) {
            throw new Error(`executable asset is not a directory: ${directory}`);
        }
        assets.add(directory);
        output.overwrite(
            location.start,
            location.end,
            JSON.stringify(`./asset/${local.split(sep).join("/")}/`),
        );
    }

    return output.toString();
}

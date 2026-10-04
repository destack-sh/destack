import { realpath, stat } from "node:fs/promises";
import { isAbsolute, relative, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parseSync, Visitor } from "rolldown/utils";
import MagicString from "magic-string";

/** A literal directory URL in a module source. */
interface DirectoryReference {
    /** The relative directory location. */
    value: string;
    /** The source offset where the literal starts. */
    start: number;
    /** The source offset where the literal ends. */
    end: number;
}

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
    const references = directoryReferences(parsed.program);
    if (!references.length) {
        return;
    }

    // retain paths under the executable root without flattening separate package assets
    const output = new MagicString(source);
    for (const location of references) {
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

/** Collect the `new URL("./directory/", import.meta.url)` references of a parsed module. */
function directoryReferences(
    program: ReturnType<typeof parseSync>["program"],
): DirectoryReference[] {
    // match directory URLs relative to the module itself
    const references: DirectoryReference[] = [];
    new Visitor({
        NewExpression(node) {
            const [location, base] = node.arguments;
            if (
                node.callee.type === "Identifier" &&
                node.callee.name === "URL" &&
                node.arguments.length === 2 &&
                location?.type === "Literal" &&
                typeof location.value === "string" &&
                /^\.{1,2}\//u.test(location.value) &&
                location.value.endsWith("/") &&
                base?.type === "MemberExpression" &&
                !base.computed &&
                base.property.type === "Identifier" &&
                base.property.name === "url" &&
                base.object.type === "MetaProperty" &&
                base.object.meta.name === "import" &&
                base.object.property.name === "meta"
            ) {
                references.push({
                    value: location.value,
                    start: location.start,
                    end: location.end,
                });
            }
        },
    }).visit(program);

    return references;
}

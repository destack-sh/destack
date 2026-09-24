import { readFile } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import MagicString from "magic-string";
import { parseAst } from "rolldown/parseAst";
import { ModuleMetadata } from "../definition/metadata.ts";
import { DECLARATION_CONSTRUCTORS } from "../declare/constructor.ts";

/** Constructors that receive their calling module, with the argument count before it. */
const STAMPED = Object.fromEntries(
    Object.entries(DECLARATION_CONSTRUCTORS).filter(
        ([, constructor]) => "arguments" in constructor,
    ),
) as Readonly<Record<string, { readonly package: string; readonly arguments: number }>>;

/** The module-local binding holding injected metadata. */
const BINDING = "__destackModule";

/** A package directory and the metadata its modules receive. */
export interface ModulePackage {
    /** The directory containing destack.json and package.json. */
    readonly directory: string;
    /** The metadata injected into each module of the package. */
    readonly metadata: ModuleMetadata;
}

/** A transformed module and its source map. */
export interface ModuleTransform {
    /** The transformed source text. */
    readonly code: string;
    /** The source map from transformed to authored text. */
    readonly map: ReturnType<MagicString["generateMap"]>;
}

/** Destack packages found above module paths, cached by directory. */
export class ModulePackages {
    /** Pending and completed lookups by directory. */
    readonly #lookups = new Map<string, Promise<ModulePackage | undefined>>();

    /** Find the Destack package containing a module, or nothing outside Destack packages. */
    find(path: string): Promise<ModulePackage | undefined> {
        return this.#lookup(dirname(path));
    }

    /** Share one lookup per directory across concurrent module loads. */
    #lookup(directory: string): Promise<ModulePackage | undefined> {
        let lookup = this.#lookups.get(directory);
        if (!lookup) {
            lookup = this.#read(directory);
            this.#lookups.set(directory, lookup);
        }

        return lookup;
    }

    /** Read the nearest package definition at or above a directory. */
    async #read(directory: string): Promise<ModulePackage | undefined> {
        // stop at dependency installations and the filesystem root
        if (basename(directory) === "node_modules" || dirname(directory) === directory) {
            return undefined;
        }

        // read both manifests where the Destack definition exists
        try {
            const definition = JSON.parse(await readFile(join(directory, "destack.json"), "utf8"));
            const manifest = JSON.parse(await readFile(join(directory, "package.json"), "utf8"));
            const metadata = ModuleMetadata.parse({
                package: { id: definition.id, name: manifest.name, version: manifest.version },
            });

            return { directory, metadata };
        }
        // continue with the parent directory when this one defines no package
        catch (error) {
            if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
                throw error;
            }

            return this.#lookup(dirname(directory));
        }
    }
}

/** Inject module metadata into import.meta.destack and declaration constructor calls. */
export function transformModule(
    code: string,
    path: string,
    metadata: ModuleMetadata,
): ModuleTransform | undefined {
    // skip modules without metadata reads or constructor names
    const names = Object.keys(STAMPED);
    if (!code.includes("import.meta.destack") && !names.some((name) => code.includes(name))) {
        return undefined;
    }

    // collect local bindings of constructors imported from their defining package
    const program = parseAst(code, { lang: /\.[cm]?tsx$/.test(path) ? "tsx" : "ts" }, path);
    const constructors = new Map<string, number>();
    for (const statement of program.body) {
        if (statement.type !== "ImportDeclaration" || statement.importKind === "type") {
            continue;
        }
        const exported = importedConstructors(statement.source.value, metadata);
        for (const specifier of statement.specifiers) {
            if (
                specifier.type === "ImportSpecifier" &&
                specifier.importKind !== "type" &&
                specifier.imported.type === "Identifier" &&
                Object.hasOwn(exported, specifier.imported.name)
            ) {
                constructors.set(specifier.local.name, exported[specifier.imported.name]);
            }
        }
    }

    // replace metadata reads and append metadata to constructor calls that omit it
    const source = new MagicString(code);
    let isChanged = false;
    visit(program, (node) => {
        if (isMetadataRead(node)) {
            source.overwrite(node.start, node.end, BINDING);
            isChanged = true;
        } else if (
            node.type === "CallExpression" &&
            node.callee.type === "Identifier" &&
            node.arguments.length === constructors.get(node.callee.name)
        ) {
            source.appendLeft(node.arguments.at(-1).end, `, ${BINDING}`);
            isChanged = true;
        }
    });
    if (!isChanged) {
        return undefined;
    }

    // declare frozen metadata before any module code runs
    source.prepend(`const ${BINDING} = Object.freeze(${JSON.stringify(metadata)});\n`);

    return { code: source.toString(), map: source.generateMap({ source: path, hires: true }) };
}

/** Return the constructors a module specifier can provide to a module of the given package. */
function importedConstructors(
    specifier: string,
    metadata: ModuleMetadata,
): Readonly<Record<string, number>> {
    // accept relative imports within the defining package, and the package with its subpaths
    const owner = specifier.startsWith(".")
        ? metadata.package.name
        : specifier.split("/").slice(0, 2).join("/");

    return Object.fromEntries(
        Object.entries(STAMPED)
            .filter(([, constructor]) => constructor.package === owner)
            .map(([name, constructor]) => [name, constructor.arguments]),
    );
}

/** A parsed AST node with source offsets. */
interface Node {
    /** The node type name. */
    readonly type: string;
    /** The offset of the first character. */
    readonly start: number;
    /** The offset after the last character. */
    readonly end: number;
    readonly [key: string]: unknown;
}

/** Report whether a node reads import.meta.destack. */
function isMetadataRead(node: Node): boolean {
    const object = node.object as Node | undefined;
    const property = node.property as (Node & { name?: string }) | undefined;

    return (
        node.type === "MemberExpression" &&
        object?.type === "MetaProperty" &&
        property?.type === "Identifier" &&
        property.name === "destack"
    );
}

/** Visit every AST node in source order, skipping children of rewritten metadata reads. */
function visit(node: unknown, callback: (node: Node & Record<string, any>) => void): void {
    // descend through arrays of nodes
    if (Array.isArray(node)) {
        for (const child of node) {
            visit(child, callback);
        }
        return;
    }

    // ignore scalar values
    if (!node || typeof node !== "object" || typeof (node as Node).type !== "string") {
        return;
    }

    // visit the node before its children
    callback(node as Node & Record<string, any>);
    if (isMetadataRead(node as Node)) {
        return;
    }
    for (const [key, value] of Object.entries(node)) {
        if (
            key !== "type" &&
            key !== "start" &&
            key !== "end" &&
            value &&
            typeof value === "object"
        ) {
            visit(value, callback);
        }
    }
}

import { readFile } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import MagicString from "magic-string";
import { parseAst } from "rolldown/parseAst";
import { ModuleMetadata } from "../definition/metadata.ts";
import type { ViewDefinition } from "../view/view.ts";
import { packageConstructors } from "./constructor.ts";

/** The module-local binding holding injected metadata. */
const BINDING = "__destackModule";

/** The module-local binding holding the metadata and views definePackage receives. */
const PACKAGE_BINDING = "__destackPackage";

/** The package whose definePackage constructor receives the package's views. */
const PACKAGE_NAME = "@destack/package";

/** A package directory and the metadata its modules receive. */
export interface ModulePackage {
    /** The directory containing destack.json and package.json. */
    readonly directory: string;
    /** The metadata injected into each module of the package. */
    readonly metadata: ModuleMetadata;
    /** The views its destack.json declares, stamped on definePackage alone. */
    readonly views: Readonly<Record<string, ViewDefinition>>;
}

/** A transformed module and its source map. */
export interface ModuleTransform {
    /** The transformed source text. */
    readonly code: string;
    /** The source map from transformed to authored text. */
    readonly map: ReturnType<MagicString["generateMap"]>;
}

/** An imported constructor's module parameter position and the binding it receives. */
interface Constructor {
    /** The parameter position of the module argument. */
    readonly position: number;
    /** The module-local binding passed at that position. */
    readonly binding: typeof BINDING | typeof PACKAGE_BINDING;
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

/** Destack packages found above module paths, cached by directory. */
export class PackageLocator {
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

            return { directory, metadata, views: definition.views ?? {} };
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
    owner: ModulePackage,
): ModuleTransform | undefined {
    // skip modules without metadata reads or constructor calls
    if (!code.includes("import.meta.destack") && !code.includes("define")) {
        return undefined;
    }

    // collect local bindings of constructors and namespaces imported from their defining package
    const program = parseAst(code, { lang: /\.[cm]?tsx$/.test(path) ? "tsx" : "ts" }, path);
    const constructors = new Map<string, Constructor>();
    const namespaces = new Map<string, Readonly<Record<string, Constructor>>>();
    for (const statement of program.body) {
        if (statement.type !== "ImportDeclaration" || statement.importKind === "type") {
            continue;
        }
        const exported = importedConstructors(statement.source.value, path, owner.metadata);
        for (const specifier of statement.specifiers) {
            // bind named constructor imports
            if (
                specifier.type === "ImportSpecifier" &&
                specifier.importKind !== "type" &&
                specifier.imported.type === "Identifier" &&
                Object.hasOwn(exported, specifier.imported.name)
            ) {
                constructors.set(specifier.local.name, exported[specifier.imported.name]!);
            }
            // bind namespaces of packages with constructors
            else if (
                specifier.type === "ImportNamespaceSpecifier" &&
                Object.keys(exported).length > 0
            ) {
                namespaces.set(specifier.local.name, exported);
            }
        }
    }

    // replace metadata reads and append metadata to constructor calls that omit it
    const source = new MagicString(code);
    const bindings = new Set<Constructor["binding"]>();
    visit(program, (node) => {
        // replace metadata reads with the module binding
        if (isMetadataRead(node)) {
            source.overwrite(node.start, node.end, BINDING);
            bindings.add(BINDING);
        }
        // append the module, or the package with its views, to constructor calls that omit it
        else {
            const binding = stampCall(node, constructors, namespaces, source);
            if (binding !== undefined) {
                bindings.add(binding);
            }
        }
    });
    if (bindings.size === 0) {
        return undefined;
    }

    // declare the frozen bindings the module uses before any module code runs
    const values = {
        [BINDING]: owner.metadata,
        [PACKAGE_BINDING]: { ...owner.metadata, views: owner.views },
    };
    for (const binding of bindings) {
        source.prepend(`const ${binding} = Object.freeze(${JSON.stringify(values[binding])});\n`);
    }

    return { code: source.toString(), map: source.generateMap({ source: path, hires: true }) };
}

/** Append a constructor's binding to its call, padding omitted optional arguments. */
function stampCall(
    node: Node & Record<string, any>,
    constructors: ReadonlyMap<string, Constructor>,
    namespaces: ReadonlyMap<string, Readonly<Record<string, Constructor>>>,
    source: MagicString,
): Constructor["binding"] | undefined {
    // skip other calls, calls passing their module explicitly, and spread arguments
    const constructor =
        node.type === "CallExpression"
            ? findConstructor(node.callee, constructors, namespaces)
            : undefined;
    const values = (node.arguments ?? []) as Node[];
    if (
        constructor === undefined ||
        values.length > constructor.position ||
        values.some((value) => value.type === "SpreadElement")
    ) {
        return undefined;
    }

    // place the binding at its parameter position
    const padding = Array.from({ length: constructor.position - values.length }, () => "undefined");
    const appended = [...padding, constructor.binding].join(", ");
    const last = values.at(-1);

    // append after the last argument
    if (last) {
        source.appendLeft(last.end, `, ${appended}`);
    }
    // insert into empty argument lists before the closing parenthesis
    else {
        source.appendLeft(node.end - 1, appended);
    }

    return constructor.binding;
}

/** Map the stamped constructors a module specifier provides to their module parameters. */
function importedConstructors(
    specifier: string,
    path: string,
    metadata: ModuleMetadata,
): Readonly<Record<string, Constructor>> {
    // read relative imports from the module's own package, and bare imports from the named package
    const owner = specifier.startsWith(".")
        ? metadata.package.name
        : specifier.startsWith("@")
          ? specifier.split("/").slice(0, 2).join("/")
          : specifier.split("/")[0]!;

    return Object.fromEntries(
        Object.entries(packageConstructors(owner, dirname(path)))
            .filter(([, constructor]) => constructor.module !== undefined)
            .map(([name, constructor]) => [
                name,
                {
                    position: constructor.module!,
                    binding:
                        owner === PACKAGE_NAME && name === "definePackage"
                            ? PACKAGE_BINDING
                            : BINDING,
                },
            ]),
    );
}

/** Find the imported constructor a callee names. */
function findConstructor(
    callee: Node & Record<string, any>,
    constructors: ReadonlyMap<string, Constructor>,
    namespaces: ReadonlyMap<string, Readonly<Record<string, Constructor>>>,
): Constructor | undefined {
    // read a named import
    if (callee.type === "Identifier") {
        return constructors.get(callee.name);
    }
    // read a namespace member
    else if (
        callee.type === "MemberExpression" &&
        !callee.computed &&
        callee.object.type === "Identifier" &&
        callee.property.type === "Identifier"
    ) {
        const exported = namespaces.get(callee.object.name);

        return exported && Object.hasOwn(exported, callee.property.name)
            ? exported[callee.property.name]
            : undefined;
    }

    return undefined;
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

    // descend into child nodes
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

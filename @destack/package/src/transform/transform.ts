import MagicString from "magic-string";
import { parseAst } from "rolldown/parseAst";
import { type ESTree, Visitor } from "rolldown/utils";
import { relative, sep } from "node:path";
import type { DeclarationSite } from "../definition/declaration.ts";
import { ModuleMetadata } from "../definition/metadata.ts";
import type { FileSystem } from "@destack/environment";
import { nth, type Version } from "@destack/schema";
import { type ModulePackage, PackageLocator, type StampedFunction } from "../locator/index.ts";

/** The module-local binding with injected metadata. */
const BINDING = "__destackModule";

/** The module recording declarations: a pass-through, except where a build evaluates them. */
export const DECLARED_MODULE = "virtual:destack-declared";

/** The pass-through declaration recorder, which leaves each declaration as it is. */
export const DECLARED_PASSTHROUGH = "export const declare = (value) => value;\n";

/** The module-local binding of the declaration recorder. */
const DECLARE = "__destackDeclare";

/** The source of each static import or re-export: one line after a `from` outside a string. */
const IMPORT_SOURCE = /(?<!["'])\bfrom\s*["']([^"'\n]+)["']/gu;

/** A local import binding of a package with stamped functions. */
interface Binding {
    /** The import's stamped functions, by export path. */
    readonly functions: ReadonlyMap<string, StampedFunction>;
    /** The export path the binding references, empty for a namespace. */
    readonly path: readonly string[];
}

/** A module's stamped text and the map to its authored text. */
export interface ModuleTransform {
    /** The stamped text. */
    readonly code: string;
    /** The map from the stamped text to the authored one. */
    readonly map: ReturnType<MagicString["generateMap"]>;
}

/** A build's module transformer: it stamps each module once per text, whichever pass reads it. */
export class ModuleTransformer {
    /** The packages the modules belong to and import stamped functions from. */
    readonly packages: PackageLocator;
    /** Each module's text and its transform, by path. */
    readonly #transformed = new Map<
        string,
        { readonly code: string; readonly result: Promise<ModuleTransform | undefined> }
    >();

    /** Stamp modules of the packages on some files, at a build's versions. */
    constructor(files: FileSystem, versions: Readonly<Record<string, Version>> = {}) {
        this.packages = new PackageLocator(files, versions);
    }

    /** Stamp a module of a Destack package, once per text, absent for one that needs no stamps. */
    transform(code: string, path: string): Promise<ModuleTransform | undefined> {
        // reuse the transform of the same text
        const previous = this.#transformed.get(path);
        if (previous?.code === code) {
            return previous.result;
        }

        // stamp the module and generate its map once
        const result = this.#transform(code, path);
        this.#transformed.set(path, { code, result });

        return result;
    }

    /** Stamp a module of a Destack package with its map. */
    async #transform(code: string, path: string): Promise<ModuleTransform | undefined> {
        // leave files outside Destack packages unchanged
        const owner = await this.packages.find(path);

        return owner === undefined
            ? undefined
            : await transformModule(code, path, owner, this.packages);
    }
}

/** Map each module-level constant's initializer, by its start, to whether its module exports it. */
function moduleBindings(program: ESTree.Program): Map<number, DeclarationSite["binding"]> {
    // list the local names an export list exports
    const listed = new Set<string>();
    for (const statement of program.body) {
        if (statement.type === "ExportNamedDeclaration" && statement.source === null) {
            for (const specifier of statement.specifiers) {
                if (specifier.local.type === "Identifier") {
                    listed.add(specifier.local.name);
                }
            }
        }
    }

    // map each constant's initializer, exported where declared or listed
    const bound = new Map<number, DeclarationSite["binding"]>();
    for (const statement of program.body) {
        const isDeclaredExported =
            statement.type === "ExportNamedDeclaration" &&
            statement.declaration?.type === "VariableDeclaration";
        const declaration = isDeclaredExported
            ? statement.declaration
            : statement.type === "VariableDeclaration"
              ? statement
              : undefined;
        for (const declarator of declaration?.type === "VariableDeclaration"
            ? declaration.declarations
            : []) {
            if (declarator.init !== null && declarator.id.type === "Identifier") {
                const isExported = isDeclaredExported || listed.has(declarator.id.name);
                bound.set(declarator.init.start, isExported ? "exported" : "unexported");
            }
        }
    }

    return bound;
}

/** Inject module metadata into import.meta.destack and the calls and tags of stamped functions. */
export async function transformModule(
    code: string,
    path: string,
    owner: ModulePackage,
    packages: PackageLocator,
): Promise<ModuleTransform | undefined> {
    // skip modules without metadata reads or imported stamped functions
    if (!(await isStampable(code, path, owner.metadata, packages))) {
        return undefined;
    }

    // collect the local bindings of imports from packages with stamped functions
    const program = parseAst(code, { lang: /\.[cm]?tsx$/u.test(path) ? "tsx" : "ts" }, path);
    const bindings = new Map<string, Binding>();
    for (const statement of program.body) {
        if (statement.type !== "ImportDeclaration" || statement.importKind === "type") {
            continue;
        }
        const functions = await packages.imported(statement.source.value, path, owner.metadata);
        if (functions.size === 0) {
            continue;
        }
        for (const specifier of statement.specifiers) {
            // bind named value imports by their exported name
            if (
                specifier.type === "ImportSpecifier" &&
                specifier.importKind !== "type" &&
                specifier.imported.type === "Identifier"
            ) {
                bindings.set(specifier.local.name, { functions, path: [specifier.imported.name] });
            }
            // bind namespaces by their members
            else if (specifier.type === "ImportNamespaceSpecifier") {
                bindings.set(specifier.local.name, { functions, path: [] });
            }
        }
    }

    // replace metadata reads, pass the module to stamped calls and tags, and record declaration sites
    const source = new MagicString(code);
    const lines = lineStarts(code);
    const bound = moduleBindings(program);
    const module = relative(owner.directory, path).split(sep).join("/");
    const recorded = new Set<number>();
    let isChanged = false;
    let isDeclaring = false;
    new Visitor({
        // replace metadata reads with the module binding
        MemberExpression(node) {
            if (isMetadataRead(node)) {
                source.overwrite(node.start, node.end, BINDING);
                isChanged = true;
            }
        },
        // append the module to stamped calls that omit it, and record declaration sites
        CallExpression(node) {
            // append the module
            if (stampCall(node, bindings, source)) {
                isChanged = true;
            }

            // record a declaration at the end of its chained calls, once
            const root = chainRoot(node);
            const stamped = findStamp(root.callee, bindings);
            if (stamped?.isDeclaration === true && !recorded.has(root.start)) {
                recorded.add(root.start);
                const site = {
                    package: stamped.owner,
                    constructor: stamped.name,
                    module,
                    ...lineColumn(lines, root.start),
                    binding: bound.get(root.start) ?? "none",
                } satisfies DeclarationSite;
                source.prependRight(node.start, `${DECLARE}(`);
                source.appendLeft(node.end, `, ${JSON.stringify(site)})`);
                isChanged = true;
                isDeclaring = true;
            }
        },
        // tag the template with the stamped tag's module form
        TaggedTemplateExpression(node) {
            const stamped = findStamp(node.tag, bindings);
            if (stamped !== undefined) {
                source.appendLeft(
                    node.tag.end,
                    `.withModule(${moduleArguments(stamped.module, 0)})`,
                );
                isChanged = true;
            }
        },
    }).visit(program);
    if (!isChanged) {
        return undefined;
    }

    // declare the frozen binding, and import the recorder of a declaring module, before any module code runs
    source.prepend(`const ${BINDING} = Object.freeze(${JSON.stringify(owner.metadata)});\n`);
    if (isDeclaring) {
        source.prepend(
            `import { declare as ${DECLARE} } from ${JSON.stringify(DECLARED_MODULE)};\n`,
        );
    }

    return {
        code: source.toString(),
        map: source.generateMap({ source: path, hires: true }),
    };
}

/** Report whether a module reads its metadata or imports a stamped function, without parsing it. */
async function isStampable(
    code: string,
    path: string,
    metadata: ModuleMetadata,
    packages: PackageLocator,
): Promise<boolean> {
    // read the metadata
    if (code.includes("import.meta.destack")) {
        return true;
    }

    // find a stamped function's root among the module's import sources
    for (const [, specifier] of code.matchAll(IMPORT_SOURCE)) {
        if (specifier === undefined) {
            throw new TypeError("the import source pattern captures no specifier");
        }
        const functions = await packages.imported(specifier, path, metadata);
        if ([...functions.keys()].some((name) => code.includes(name.split(".", 1)[0] ?? name))) {
            return true;
        }
    }

    return false;
}

/** Append the module binding to a stamped call, padding omitted arguments, reporting success. */
function stampCall(
    node: ESTree.CallExpression,
    bindings: ReadonlyMap<string, Binding>,
    source: MagicString,
): boolean {
    // skip other calls, calls passing their module explicitly, and spread arguments
    const stamped = findStamp(node.callee, bindings);
    const values = node.arguments;
    if (
        stamped === undefined ||
        values.length > stamped.module ||
        values.some((value) => value.type === "SpreadElement")
    ) {
        return false;
    }

    // append after the last argument
    const appended = moduleArguments(stamped.module, values.length);
    const last = values.at(-1);
    if (last) {
        source.appendLeft(last.end, `, ${appended}`);
    }
    // insert into empty argument lists before the closing parenthesis
    else {
        source.appendLeft(node.end - 1, appended);
    }

    return true;
}

/** Find the stamped function an import or member chain reads. */
function findStamp(
    expression: ESTree.Expression,
    bindings: ReadonlyMap<string, Binding>,
): StampedFunction | undefined {
    // collect the member names down to the root identifier
    const members: string[] = [];
    let current = expression;
    while (
        current.type === "MemberExpression" &&
        !current.computed &&
        current.property.type === "Identifier"
    ) {
        members.unshift(current.property.name);
        current = current.object;
    }
    if (current.type !== "Identifier") {
        return undefined;
    }

    // resolve the root's import to the export path
    const binding = bindings.get(current.name);

    return binding?.functions.get([...binding.path, ...members].join("."));
}

/** Find the first call of a chain of calls, such as `defineY` in `defineY().implement()`. */
function chainRoot(node: ESTree.CallExpression): ESTree.CallExpression {
    // follow each call's callee to the call it chains on
    let root = node;
    while (
        root.callee.type === "MemberExpression" &&
        root.callee.object.type === "CallExpression"
    ) {
        root = root.callee.object;
    }

    return root;
}

/** List the offset each line of a source starts at. */
function lineStarts(code: string): number[] {
    const starts = [0];
    for (let index = code.indexOf("\n"); index >= 0; index = code.indexOf("\n", index + 1)) {
        starts.push(index + 1);
    }

    return starts;
}

/** Read the zero-based line and column of an offset. */
function lineColumn(lines: readonly number[], offset: number): { line: number; column: number } {
    // find the last line starting at or before the offset
    let low = 0;
    let high = lines.length - 1;
    while (low < high) {
        const middle = Math.ceil((low + high) / 2);
        if (nth(lines, middle) <= offset) {
            low = middle;
        } else {
            high = middle - 1;
        }
    }

    return { line: low, column: offset - nth(lines, low) };
}

/** Write the arguments placing the module binding at its position after the given ones. */
function moduleArguments(position: number, given: number): string {
    const padding = Array.from({ length: position - given }, () => "undefined");

    return [...padding, BINDING].join(", ");
}

/** Report whether a member expression reads import.meta.destack. */
function isMetadataRead(node: ESTree.MemberExpression): boolean {
    return (
        !node.computed &&
        node.object.type === "MetaProperty" &&
        node.property.type === "Identifier" &&
        node.property.name === "destack"
    );
}

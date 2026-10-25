import type { FileSystem } from "@destack/environment";
import { formatSource } from "@destack/check";
import { dirname, join, resolve } from "node:path";
import { DependencyName, Package, PackageDefinition } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { TemplateInput } from "@destack/package/template";
import { PackageError } from "@destack/package/error";
import { type ESTree, parseSync, Visitor } from "rolldown/utils";
import { schema, Text } from "@destack/schema";

/** The fields of a template's package.json that an instance reads and rewrites. */
const TemplateManifest = schema.looseObject({
    name: schema.string(),
    version: schema.string(),
    dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
});
/** The fields of a template's package.json that an instance reads and rewrites. */
type TemplateManifest = schema.Infer<typeof TemplateManifest>;

/** Source files copied and renamed when creating a package. */
export class Template {
    /** The template's source files, including its manifests. */
    readonly files: ReadonlyMap<string, Uint8Array>;

    /** Retain a template's source files. */
    constructor(files: ReadonlyMap<string, Uint8Array>) {
        this.files = files;
    }

    /** Read declared template files without following symbolic links. */
    static async read(directory: string, files: FileSystem): Promise<Template> {
        // require regular manifests before reading the copy declaration
        const root = resolve(directory);
        const contents = new Map<string, Uint8Array>();
        for (const name of ["package.json", "destack.json"]) {
            if ((await files.lstat(join(root, name))).kind !== "file") {
                throw PackageError.of(
                    "INVALID_FILE",
                    `template manifest is not a regular file: ${name}`,
                );
            }
            contents.set(name, await files.readFile(join(root, name)));
        }
        const definition = PackageDefinition.read(
            new TextDecoder("utf-8", { fatal: true }).decode(contents.get("destack.json")),
        );
        if (!definition.template) {
            throw PackageError.of("INVALID_DEFINITION", "package has no template declaration");
        }

        // traverse declared paths while checking every parent for symbolic links
        const declared = await readDeclared(root, definition.template.files, files);
        for (const [path, bytes] of declared) {
            contents.set(path, bytes);
        }

        return new Template(contents);
    }

    /** Copy source and replace declared dependencies without executing package code. */
    async instantiate(parameters: TemplateInput): Promise<Map<string, Uint8Array>> {
        // read declarations from the selected source package
        const source = this.files;
        const { manifest, definition } = this.#readDeclarations();
        const template = definition.template;
        if (!template) {
            throw PackageError.of("INVALID_DEFINITION", "package has no template declaration");
        }
        parameters = TemplateInput.parse(parameters);

        // assign a distinct identity to the new package
        if (parameters.id === definition.id) {
            throw PackageError.of(
                "INVALID_DEFINITION",
                "template instance requires a new package ID",
            );
        }

        // replace the declared dependency selections
        if (definition.language !== "typescript") {
            throw PackageError.of(
                "UNSUPPORTED_LANGUAGE",
                "template generation requires TypeScript",
            );
        }
        replaceDependencies(manifest, template.dependencies ?? [], parameters);
        const imports = {
            ...parameters.dependencies,
            [manifest.name]: {
                id: parameters.id,
                name: parameters.name,
                version: manifest.version,
            },
        };

        // copy declared bytes and edit only parsed module specifiers
        const files = copyFiles(source, template.files, imports);

        // generate an ordinary package without retaining template classification
        const encoder = new TextEncoder();
        manifest.name = parameters.name;
        definition.id = parameters.id;
        delete definition.template;
        files.set("package.json", encoder.encode(JSON.stringify(manifest, null, 4) + "\n"));
        files.set("destack.json", encoder.encode(JSON.stringify(definition, null, 4) + "\n"));

        // format generated and copied text so new packages start formatted
        await formatFiles(files);

        return files;
    }

    /** Read the template's package manifest and definition, requiring a valid package identity. */
    #readDeclarations(): { manifest: TemplateManifest; definition: PackageDefinition } {
        // read the package manifest
        const decoder = new TextDecoder("utf-8", { fatal: true });
        const manifest: unknown = JSON.parse(
            decoder.decode(requiredFile(this.files, "package.json")),
        );
        if (!isTemplateManifest(manifest)) {
            throw PackageError.of("INVALID_DEFINITION", "template package.json is invalid");
        }

        // read the definition and check the package identity
        const definition = PackageDefinition.read(
            decoder.decode(requiredFile(this.files, "destack.json")),
        );
        Package.parse({ id: definition.id, name: manifest.name, version: manifest.version });

        return { manifest, definition };
    }

    /** Instantiate a package into a new directory. */
    async write(directory: string, parameters: TemplateInput, files: FileSystem): Promise<void> {
        // instantiate the files
        const contents = await this.instantiate(parameters);
        // validate relative paths before creating the destination
        for (const path of contents.keys()) {
            PackagePath.parse(path);
        }
        const root = resolve(directory);
        await files.mkdir(root);

        // preserve binary contents and refuse existing files
        for (const [path, bytes] of contents) {
            const target = resolve(root, path);
            await files.mkdir(dirname(target), { recursive: true });
            await files.writeFile(target, bytes, { createNew: true });
        }
    }
}

/** Read the files of declared template paths, refusing symbolic links and file parents. */
async function readDeclared(
    root: string,
    declared: readonly string[],
    files: FileSystem,
): Promise<Map<string, Uint8Array>> {
    // walk the declared paths depth first
    const contents = new Map<string, Uint8Array>();
    const pending = [...declared];
    const visited = new Set<string>();
    for (let path = pending.pop(); path !== undefined; path = pending.pop()) {
        // visit each path once
        if (visited.has(path)) {
            continue;
        }
        visited.add(path);

        // require every parent to be a directory
        const segments = path.split("/");
        for (let count = 1; count < segments.length; count++) {
            const parent = await files.lstat(join(root, ...segments.slice(0, count)));
            if (parent.kind !== "directory") {
                throw PackageError.of(
                    "INVALID_FILE",
                    `template parent is not a directory: ${path}`,
                );
            }
        }

        // descend into a directory, or read a regular file
        const absolute = join(root, path);
        const entry = await files.lstat(absolute);
        if (entry.kind === "directory") {
            for (const { name } of await files.readDirectory(absolute)) {
                pending.push(path + "/" + name);
            }
        } else if (entry.kind === "file") {
            contents.set(path, await files.readFile(absolute));
        } else {
            throw PackageError.of("INVALID_FILE", `unsupported template file: ${path}`);
        }
    }

    return contents;
}

/** Replace a template manifest's declared dependency selections with the chosen packages. */
function replaceDependencies(
    manifest: TemplateManifest,
    declared: readonly string[],
    parameters: TemplateInput,
): void {
    // require all declared selections and reject unrelated overrides
    const expected = [...declared].toSorted();
    const selected = Object.keys(parameters.dependencies).toSorted();
    if (JSON.stringify(expected) !== JSON.stringify(selected)) {
        throw PackageError.of(
            "INVALID_DEPENDENCY",
            "template dependencies do not match the declared selections",
        );
    }

    // remove each declared selection
    const dependencies = manifest.dependencies ?? {};
    for (const original of expected) {
        if (typeof dependencies[original] !== "string") {
            throw PackageError.of("INVALID_DEPENDENCY", `missing template dependency: ${original}`);
        }
        delete dependencies[original];
    }

    // add each chosen package, refusing a conflicting requirement
    for (const replacement of Object.values(parameters.dependencies)) {
        const existing = dependencies[replacement.name];
        if (
            replacement.name === parameters.name ||
            (existing !== undefined && existing !== replacement.version)
        ) {
            throw PackageError.of(
                "INVALID_DEPENDENCY",
                `conflicting template dependency: ${replacement.name}`,
            );
        }
        dependencies[replacement.name] = replacement.version;
    }
}

/** Copy the template's declared files in path order, rewriting module specifiers in scripts. */
function copyFiles(
    source: ReadonlyMap<string, Uint8Array>,
    declared: readonly string[],
    imports: Readonly<Record<string, Package>>,
): Map<string, Uint8Array> {
    // copy each declared file
    const decoder = new TextDecoder("utf-8", { fatal: true });
    const encoder = new TextEncoder();
    const files = new Map<string, Uint8Array>();
    for (const [path, bytes] of [...source].toSorted(([left], [right]) =>
        Text.compare(left, right),
    )) {
        PackagePath.parse(path);
        if (!declared.some((entry) => isWithin(path, entry))) {
            continue;
        }
        const contents = /\.[cm]?[jt]sx?$/u.test(path)
            ? encoder.encode(replaceImports(path, decoder.decode(bytes), imports))
            : bytes.slice();
        files.set(path, contents);
    }

    // require every declared path and both manifests
    for (const entry of declared) {
        if (![...files.keys()].some((path) => isWithin(path, entry))) {
            throw PackageError.of("INVALID_FILE", `missing template source: ${entry}`);
        }
    }
    requiredFile(files, "package.json");
    requiredFile(files, "destack.json");

    return files;
}

/** Format each text file of a new package in place. */
async function formatFiles(files: Map<string, Uint8Array>): Promise<void> {
    const encoder = new TextEncoder();
    const decoder = new TextDecoder("utf-8", { fatal: true });
    for (const [path, bytes] of files) {
        if (/\.(?:[cm]?[jt]sx?|json|css|md|html)$/u.test(path)) {
            files.set(path, encoder.encode(await formatSource(path, decoder.decode(bytes))));
        }
    }
}

/** Report whether a path is a declared template path or lies below it. */
function isWithin(path: string, entry: string): boolean {
    return path === entry || path.startsWith(entry + "/");
}

/** Read a required source file. */
function requiredFile(files: ReadonlyMap<string, Uint8Array>, path: string): Uint8Array {
    const bytes = files.get(path);
    if (!bytes) {
        throw PackageError.of("INVALID_FILE", `missing template file: ${path}`);
    }

    return bytes;
}

/** Rewrite module references while preserving surrounding source text. */
export function replaceImports(
    path: string,
    source: string,
    dependencies: Readonly<Record<string, Package>>,
): string {
    // reject malformed source before selecting edits
    const parsed = parseSync(path, source);
    if (parsed.errors.length) {
        throw PackageError.of("INVALID_FILE", `invalid template source: ${path}`);
    }
    const references: ESTree.StringLiteral[] = [];
    const visitor = new Visitor({
        ImportDeclaration: (node) => {
            references.push(node.source);
        },
        ExportNamedDeclaration: (node) => {
            if (node.source) {
                references.push(node.source);
            }
        },
        ExportAllDeclaration: (node) => {
            references.push(node.source);
        },
        TSImportType: (node) => {
            references.push(node.source);
        },
        ImportExpression: (node) => {
            if (node.source.type === "Literal" && typeof node.source.value === "string") {
                references.push(node.source);
            } else {
                throw PackageError.of(
                    "INVALID_FILE",
                    `template imports must use literal specifiers: ${path}`,
                );
            }
        },
    });
    visitor.visit(parsed.program);

    // apply edits from the end, so parser offsets stay valid
    for (const reference of references.toSorted((left, right) => right.start - left.start)) {
        const name = DependencyName.of(reference.value);
        const replacement = dependencies[name];
        if (!replacement) {
            continue;
        }
        const specifier = replacement.name + reference.value.slice(name.length);
        const text = JSON.stringify(specifier);
        source = source.slice(0, reference.start) + text + source.slice(reference.end);
    }

    return source;
}

/** Report whether a parsed package.json has a template's fields, preserving its key order. */
function isTemplateManifest(value: unknown): value is TemplateManifest {
    return TemplateManifest.safeParse(value).success;
}

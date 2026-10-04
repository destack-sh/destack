import { lstat, mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { formatSource } from "@destack/check";
import { dirname, join, resolve } from "node:path";
import { DependencyName, Package, PackageDefinition } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { TemplateInput } from "@destack/package/template";
import { PackageError } from "@destack/package/error";
import { type ESTree, parseSync, Visitor } from "rolldown/utils";
import { schema } from "@destack/schema";
import { compareText } from "../build/serialization.ts";

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
    static async read(directory: string): Promise<Template> {
        // require regular manifests before reading the copy declaration
        const root = resolve(directory);
        const files = new Map<string, Uint8Array>();
        for (const name of ["package.json", "destack.json"]) {
            if (!(await lstat(join(root, name))).isFile()) {
                throw new PackageError(
                    "INVALID_FILE",
                    `template manifest is not a regular file: ${name}`,
                );
            }
            files.set(name, new Uint8Array(await readFile(join(root, name))));
        }
        const definition = PackageDefinition.read(
            new TextDecoder("utf-8", { fatal: true }).decode(files.get("destack.json")),
        );
        if (!definition.template) {
            throw new PackageError("INVALID_DEFINITION", "package has no template declaration");
        }

        // traverse declared paths while checking every parent for symbolic links
        for (const [path, bytes] of await readDeclared(root, definition.template.files)) {
            files.set(path, bytes);
        }

        return new Template(files);
    }

    /** Copy source and replace declared dependencies without executing package code. */
    async instantiate(parameters: TemplateInput): Promise<Map<string, Uint8Array>> {
        // read declarations from the selected source package
        const source = this.files;
        const { manifest, definition } = this.#readDeclarations();
        const template = definition.template;
        if (!template) {
            throw new PackageError("INVALID_DEFINITION", "package has no template declaration");
        }
        parameters = TemplateInput.parse(parameters);

        // assign a distinct identity to the new package
        if (parameters.id === definition.id) {
            throw new PackageError(
                "INVALID_DEFINITION",
                "template instance requires a new package ID",
            );
        }

        // replace the declared dependency selections
        if (definition.language !== "typescript") {
            throw new PackageError(
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
            throw new PackageError("INVALID_DEFINITION", "template package.json is invalid");
        }

        // read the definition and check the package identity
        const definition = PackageDefinition.read(
            decoder.decode(requiredFile(this.files, "destack.json")),
        );
        Package.parse({ id: definition.id, name: manifest.name, version: manifest.version });

        return { manifest, definition };
    }

    /** Instantiate a package into a new directory. */
    async write(directory: string, parameters: TemplateInput): Promise<void> {
        // instantiate the files
        const files = await this.instantiate(parameters);
        // validate relative paths before creating the destination
        for (const path of files.keys()) {
            PackagePath.parse(path);
        }
        const root = resolve(directory);
        await mkdir(root);

        // preserve binary contents and refuse existing files
        for (const [path, bytes] of files) {
            const target = resolve(root, path);
            await mkdir(dirname(target), { recursive: true });
            await writeFile(target, bytes, { flag: "wx" });
        }
    }
}

/** Read the files of declared template paths, refusing symbolic links and parents that are not directories. */
async function readDeclared(
    root: string,
    declared: readonly string[],
): Promise<Map<string, Uint8Array>> {
    // walk the declared paths depth first
    const files = new Map<string, Uint8Array>();
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
            if (!(await lstat(join(root, ...segments.slice(0, count)))).isDirectory()) {
                throw new PackageError(
                    "INVALID_FILE",
                    `template parent is not a directory: ${path}`,
                );
            }
        }

        // descend into a directory, or read a regular file
        const absolute = join(root, path);
        const entry = await lstat(absolute);
        if (entry.isDirectory()) {
            for (const name of await readdir(absolute)) {
                pending.push(path + "/" + name);
            }
        } else if (entry.isFile()) {
            files.set(path, new Uint8Array(await readFile(absolute)));
        } else {
            throw new PackageError("INVALID_FILE", `unsupported template file: ${path}`);
        }
    }

    return files;
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
        throw new PackageError(
            "INVALID_DEPENDENCY",
            "template dependencies do not match the declared selections",
        );
    }

    // remove each declared selection
    const dependencies = manifest.dependencies ?? {};
    for (const original of expected) {
        if (typeof dependencies[original] !== "string") {
            throw new PackageError(
                "INVALID_DEPENDENCY",
                `missing template dependency: ${original}`,
            );
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
            throw new PackageError(
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
        compareText(left, right),
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
            throw new PackageError("INVALID_FILE", `missing template source: ${entry}`);
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
        throw new PackageError("INVALID_FILE", `missing template file: ${path}`);
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
        throw new PackageError("INVALID_FILE", `invalid template source: ${path}`);
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
                throw new PackageError(
                    "INVALID_FILE",
                    `template imports must use literal specifiers: ${path}`,
                );
            }
        },
    });
    visitor.visit(parsed.program);

    // apply edits from the end to keep parser offsets valid
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

/** Report whether a parsed package.json has a template's fields, keeping the parsed object and its key order. */
function isTemplateManifest(value: unknown): value is TemplateManifest {
    return TemplateManifest.safeParse(value).success;
}

import { lstat, mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { PackageDefinition } from "@destack/package";
import { Package } from "@destack/package/package";
import { PackagePath } from "@destack/package/file";
import { TemplateParameters } from "@destack/package/template";
import { PackageError } from "@destack/package/error";
import { type ESTree, parseSync, Visitor } from "rolldown/utils";

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
                    `Template manifest is not a regular file: ${name}`,
                );
            }
            files.set(name, new Uint8Array(await readFile(join(root, name))));
        }
        const definition = PackageDefinition.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(files.get("destack.json"))),
        );
        if (!definition.template) {
            throw new PackageError("INVALID_DEFINITION", "Package has no template declaration.");
        }

        // traverse declared paths while checking every parent for symbolic links
        const pending = [...definition.template.files];
        const visited = new Set<string>();
        while (pending.length > 0) {
            const path = pending.pop()!;
            if (visited.has(path)) {
                continue;
            }
            visited.add(path);
            const segments = path.split("/");
            for (let count = 1; count < segments.length; count++) {
                if (!(await lstat(join(root, ...segments.slice(0, count)))).isDirectory()) {
                    throw new PackageError(
                        "INVALID_FILE",
                        `Template parent is not a directory: ${path}`,
                    );
                }
            }
            const absolute = join(root, path);
            const entry = await lstat(absolute);
            if (entry.isDirectory()) {
                for (const name of await readdir(absolute)) {
                    pending.push(path + "/" + name);
                }
            } else if (entry.isFile()) {
                files.set(path, new Uint8Array(await readFile(absolute)));
            } else {
                throw new PackageError("INVALID_FILE", `Unsupported template file: ${path}`);
            }
        }

        return new Template(files);
    }

    /** Copy source and replace declared dependencies without executing package code. */
    instantiate(parameters: TemplateParameters): Map<string, Uint8Array> {
        const source = this.files;

        // read declarations from the selected source package
        const decoder = new TextDecoder("utf-8", { fatal: true });
        const manifest = JSON.parse(decoder.decode(requiredFile(source, "package.json")));
        const definition = JSON.parse(decoder.decode(requiredFile(source, "destack.json")));
        const template = PackageDefinition.parse(definition).template;
        if (!template) {
            throw new PackageError("INVALID_DEFINITION", "Package has no template declaration.");
        }
        Package.parse({ id: definition.id, name: manifest.name, version: manifest.version });
        parameters = TemplateParameters.parse(parameters);

        // assign a distinct identity to the new package
        if (parameters.id === definition.id) {
            throw new PackageError(
                "INVALID_DEFINITION",
                "template instance requires a new package ID",
            );
        }

        // require all declared selections and reject unrelated overrides
        const expected = [...(template.dependencies ?? [])].sort();
        const selected = Object.keys(parameters.dependencies).sort();
        if (definition.language !== "typescript") {
            throw new PackageError(
                "UNSUPPORTED_LANGUAGE",
                "Template generation requires TypeScript.",
            );
        }
        if (JSON.stringify(expected) !== JSON.stringify(selected)) {
            throw new PackageError(
                "INVALID_DEPENDENCY",
                "Template dependencies do not match the declared selections.",
            );
        }
        for (const original of expected) {
            if (typeof manifest.dependencies?.[original] !== "string") {
                throw new PackageError(
                    "INVALID_DEPENDENCY",
                    `Missing template dependency: ${original}`,
                );
            }
            delete manifest.dependencies[original];
        }
        for (const replacement of Object.values(parameters.dependencies)) {
            const existing = manifest.dependencies[replacement.name];
            if (
                replacement.name === parameters.name ||
                (existing && existing !== replacement.version)
            ) {
                throw new PackageError(
                    "INVALID_DEPENDENCY",
                    `Conflicting template dependency: ${replacement.name}`,
                );
            }
            manifest.dependencies[replacement.name] = replacement.version;
        }
        const imports = {
            ...parameters.dependencies,
            [manifest.name]: {
                id: parameters.id,
                name: parameters.name,
                version: manifest.version,
            },
        };

        // copy declared bytes and edit only parsed module specifiers
        const encoder = new TextEncoder();
        const files = new Map<string, Uint8Array>();
        for (const [path, bytes] of [...source].sort(([left], [right]) =>
            left < right ? -1 : left > right ? 1 : 0,
        )) {
            PackagePath.parse(path);
            if (!template.files.some((entry) => path === entry || path.startsWith(entry + "/"))) {
                continue;
            }
            const contents = /\.[cm]?[jt]sx?$/.test(path)
                ? encoder.encode(replaceImports(path, decoder.decode(bytes), imports))
                : bytes.slice();
            files.set(path, contents);
        }
        for (const entry of template.files) {
            if (![...files.keys()].some((path) => path === entry || path.startsWith(entry + "/"))) {
                throw new PackageError("INVALID_FILE", `Missing template source: ${entry}`);
            }
        }
        requiredFile(files, "package.json");
        requiredFile(files, "destack.json");

        // generate an ordinary package without retaining template classification
        manifest.name = parameters.name;
        definition.id = parameters.id;
        delete definition.template;
        files.set("package.json", encoder.encode(JSON.stringify(manifest, null, 4) + "\n"));
        files.set("destack.json", encoder.encode(JSON.stringify(definition, null, 4) + "\n"));

        return files;
    }

    /** Instantiate a package into a new directory. */
    async write(directory: string, parameters: TemplateParameters): Promise<void> {
        const files = this.instantiate(parameters);
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

/** Read a required source file. */
function requiredFile(files: ReadonlyMap<string, Uint8Array>, path: string): Uint8Array {
    const bytes = files.get(path);
    if (!bytes) {
        throw new PackageError("INVALID_FILE", `Missing template file: ${path}`);
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
        throw new PackageError("INVALID_FILE", `Invalid template source: ${path}`);
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
                    `Template imports must use literal specifiers: ${path}`,
                );
            }
        },
    });
    visitor.visit(parsed.program);

    // apply edits from the end to keep parser offsets valid
    for (const reference of references.sort((left, right) => right.start - left.start)) {
        const name = reference.value.startsWith("@")
            ? reference.value.split("/").slice(0, 2).join("/")
            : reference.value.split("/")[0];
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

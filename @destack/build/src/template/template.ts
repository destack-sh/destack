import type { FileSystem } from "@destack/environment";
import { formatSource } from "@destack/check";
import type { Toolchain } from "@destack/check/toolchain";
import { dirname, join, resolve } from "node:path";
import {
    DependencyName,
    PackageDefinition,
    PackageJson,
    PackageName,
    PackageVersion,
} from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { TemplateInput } from "@destack/package/template";
import { PackageError } from "@destack/package/error";
import { type ESTree, parseSync, Visitor } from "rolldown/utils";
import { present, schema, Text, Version } from "@destack/schema";

/** The fields of a template's package.json that an instance reads and rewrites. */
const TemplateManifest = schema.looseObject({
    name: schema.string(),
    dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    exports: schema.record(schema.string(), schema.unknown()).exactOptional(),
});

/** The export of an app's package module, whose definition carries the package's title. */
const PACKAGE_EXPORT = "./package";
/** The fields of a template's package.json that an instance reads and rewrites. */
type TemplateManifest = schema.Infer<typeof TemplateManifest>;

/** A package file's dependencies by range. */
const DEPENDENCIES = schema.record(schema.string(), schema.string());

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
                    "FILE_INVALID",
                    `template manifest '${name}' is invalid: it is not a regular file`,
                );
            }
            contents.set(name, await files.readFile(join(root, name)));
        }
        const definition = PackageDefinition.read(
            new TextDecoder("utf-8", { fatal: true }).decode(contents.get("destack.json")),
        );
        if (!definition.template) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                "template declaration is invalid: the package has none",
            );
        }

        // traverse declared paths while checking every parent for symbolic links
        const declared = await readDeclared(root, definition.template.files, files);
        for (const [path, bytes] of declared) {
            contents.set(path, bytes);
        }

        return new Template(contents);
    }

    /** Read the dependencies an instance selects packages for. */
    get dependencies(): readonly string[] {
        return this.#readDeclarations().definition.template?.dependencies ?? [];
    }

    /** Read the dependencies the template requires from its own workspace. */
    get workspace(): readonly string[] {
        return PackageJson.workspace(schema.jsonObject().parse(this.#readDeclarations().manifest));
    }

    /** Copy source and replace declared dependencies without executing package code. */
    async instantiate(
        parameters: TemplateInput,
        toolchain: Toolchain,
    ): Promise<Map<string, Uint8Array>> {
        // read declarations from the selected source package
        const source = this.files;
        const { manifest, definition } = this.#readDeclarations();
        const template = definition.template;
        if (!template) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                "template declaration is invalid: the package has none",
            );
        }
        parameters = TemplateInput.parse(parameters);

        // assign a distinct identity to the new package
        if (parameters.id === definition.id) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                "template instance is invalid: it requires a new package ID",
            );
        }

        // replace the declared dependency selections
        if (definition.language !== "typescript") {
            throw PackageError.of(
                "LANGUAGE_INVALID",
                "template language is invalid: template generation requires TypeScript",
            );
        }
        replaceDependencies(manifest, template.dependencies ?? [], parameters);
        const imports = {
            ...parameters.dependencies,
            [manifest.name]: {
                id: parameters.id,
                name: parameters.name,
                version: Version.development,
            },
        };

        // copy declared bytes and edit only parsed module specifiers
        const files = copyFiles(source, template.files, imports);

        // title the package after its name, as the template's title names the template
        const module = packageModule(manifest);
        const decoder = new TextDecoder();
        const copied = present(files.get(module), `the template's package module '${module}'`);
        const title = titleOf(parameters.name);
        files.set(
            module,
            new TextEncoder().encode(replaceTitle(module, decoder.decode(copied), title)),
        );

        // generate an ordinary source package: no template classification, no release's version, released ranges
        const encoder = new TextEncoder();
        manifest.name = parameters.name;
        delete manifest["version"];
        definition.id = parameters.id;
        delete definition.template;
        const released = PackageJson.release(
            schema.jsonObject().parse(manifest),
            parameters.versions,
        );
        files.set("package.json", encoder.encode(JSON.stringify(released, null, 4) + "\n"));

        // declare the scopes of its Destack dependencies
        const own = `@${PackageName.split(parameters.name).handle}`;
        const depended = Object.keys(DEPENDENCIES.parse(released["dependencies"] ?? {}));
        const scopes = [
            ...new Set(
                depended
                    .filter((name) => Object.hasOwn(parameters.versions, name))
                    .map((name) => `@${PackageName.split(name).handle}`),
            ),
        ].filter((scope) => scope !== own);
        if (scopes.length > 0) {
            definition.scopes = scopes.toSorted();
        }
        files.set("destack.json", encoder.encode(JSON.stringify(definition, null, 4) + "\n"));

        // start the package's README with what its manifest says: its name and description
        const description = schema.string().min(1).parse(manifest["description"]);
        files.set("README.md", encoder.encode(`# ${parameters.name}\n\n${description}\n`));

        // format generated and copied text so new packages start formatted
        await formatFiles(files, toolchain);

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
            throw PackageError.of(
                "DEFINITION_INVALID",
                "template package.json is invalid: it does not match its schema",
            );
        }

        // read the definition and check the package identity
        const definition = PackageDefinition.read(
            decoder.decode(requiredFile(this.files, "destack.json")),
        );
        PackageVersion.parse({
            id: definition.id,
            name: manifest.name,
            version: Version.development,
        });

        return { manifest, definition };
    }

    /** Instantiate a package into a directory, refusing existing files. */
    async write(
        directory: string,
        parameters: TemplateInput,
        toolchain: Toolchain,
        files: FileSystem,
    ): Promise<void> {
        // instantiate the files
        const contents = await this.instantiate(parameters, toolchain);
        // validate relative paths before creating the destination
        for (const path of contents.keys()) {
            PackagePath.parse(path);
        }
        const root = resolve(directory);
        await files.mkdir(root, { recursive: true });

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
                    "FILE_INVALID",
                    `template parent of '${path}' is invalid: it is not a directory`,
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
            throw PackageError.of(
                "FILE_INVALID",
                `template file '${path}' is invalid: it is not supported`,
            );
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
            "DEPENDENCY_INVALID",
            "template dependency set is invalid: it does not match the declared selections",
        );
    }

    // remove each declared selection
    const dependencies = manifest.dependencies ?? {};
    for (const original of expected) {
        if (typeof dependencies[original] !== "string") {
            throw PackageError.of(
                "DEPENDENCY_INVALID",
                `template dependency '${original}' is invalid: it is missing`,
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
            throw PackageError.of(
                "DEPENDENCY_INVALID",
                `template dependency '${replacement.name}' is invalid: it conflicts`,
            );
        }
        dependencies[replacement.name] = replacement.version;
    }
}

/** Copy the template's declared files in path order, rewriting module specifiers in scripts. */
function copyFiles(
    source: ReadonlyMap<string, Uint8Array>,
    declared: readonly string[],
    imports: Readonly<Record<string, PackageVersion>>,
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
            throw PackageError.of(
                "FILE_INVALID",
                `template source '${entry}' is invalid: it is missing`,
            );
        }
    }
    requiredFile(files, "package.json");
    requiredFile(files, "destack.json");

    return files;
}

/** Format each text file of a new package in place with a toolchain's formatter. */
async function formatFiles(files: Map<string, Uint8Array>, toolchain: Toolchain): Promise<void> {
    const encoder = new TextEncoder();
    const decoder = new TextDecoder("utf-8", { fatal: true });
    for (const [path, bytes] of files) {
        if (/\.(?:[cm]?[jt]sx?|json|css|md|html)$/u.test(path)) {
            files.set(
                path,
                encoder.encode(await formatSource(path, decoder.decode(bytes), toolchain)),
            );
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
        throw PackageError.of("FILE_INVALID", `template file '${path}' is invalid: it is missing`);
    }

    return bytes;
}

/** Read the path of a template's package module, the file its manifest exports as ./package. */
function packageModule(manifest: TemplateManifest): string {
    const exported = manifest.exports?.[PACKAGE_EXPORT];
    if (typeof exported !== "string") {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `template '${manifest.name}' is invalid: it exports no ${PACKAGE_EXPORT} module`,
        );
    }

    return exported.replace(/^\.\//u, "");
}

/** Write a package's title from its name: its local name in sentence case, split at hyphens. */
function titleOf(name: string): string {
    const words = PackageName.split(name).local.replaceAll("-", " ");

    return `${words.charAt(0).toUpperCase()}${words.slice(1)}`;
}

/** Replace the one literal title of a module's definePackage call. */
export function replaceTitle(path: string, source: string, title: string): string {
    // find the title of each definePackage call
    const parsed = parseSync(path, source);
    const titles: ESTree.TemplateLiteral[] = [];
    const visitor = new Visitor({
        CallExpression: (node) => {
            // read the literal titles of a definePackage call's definition
            const [argument] = node.arguments;
            const isDefinition =
                node.callee.type === "Identifier" && node.callee.name === "definePackage";
            if (!isDefinition || argument?.type !== "ObjectExpression") {
                return;
            }
            for (const property of argument.properties) {
                const isTitle =
                    property.type === "Property" &&
                    property.key.type === "Identifier" &&
                    property.key.name === "title";
                const value = isTitle ? property.value : undefined;
                if (
                    value?.type === "TaggedTemplateExpression" &&
                    value.quasi.expressions.length === 0
                ) {
                    titles.push(value.quasi);
                }
            }
        },
    });
    visitor.visit(parsed.program);

    // refuse a module without exactly one literal title
    const [element] = titles;
    if (parsed.errors.length > 0 || titles.length !== 1 || element === undefined) {
        throw PackageError.of(
            "FILE_INVALID",
            `template source '${path}' is invalid: it must define its package with one literal title`,
        );
    }

    // write the title in place of the template's, escaped as template literal text
    const escaped = title.replaceAll("\\", "\\\\").replaceAll("`", "\\`").replaceAll("${", "\\${");

    return `${source.slice(0, element.start)}\`${escaped}\`${source.slice(element.end)}`;
}

/** Rewrite module references while preserving surrounding source text. */
export function replaceImports(
    path: string,
    source: string,
    dependencies: Readonly<Record<string, PackageVersion>>,
): string {
    // reject malformed source before selecting edits
    const parsed = parseSync(path, source);
    if (parsed.errors.length) {
        throw PackageError.of(
            "FILE_INVALID",
            `template source '${path}' is invalid: it does not parse`,
        );
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
                    "FILE_INVALID",
                    `template source '${path}' is invalid: its imports must use literal specifiers`,
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

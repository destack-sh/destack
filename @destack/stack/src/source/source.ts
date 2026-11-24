import { dirname, join, normalize } from "node:path";
import type {
    Directive,
    ImportDeclaration,
    ObjectExpression,
    ObjectProperty,
    Program,
    Span,
    Statement,
} from "@oxc-project/types";
import MagicString from "magic-string";
import { parseAst } from "rolldown/parseAst";
import { schema } from "@destack/schema";
import { StackError } from "../error/index.ts";
import type { EditOperation } from "../object/edit.ts";

/** The manifest of a package, relative to its directory. */
const MANIFEST = "package.json";

/** The function stacks are declared with. */
const DEFINE_STACK = "defineStack";

/** The function installations are declared with. */
const INSTALL = "install";

/** The export of a package its installations import, as `@scope/name/package`. */
const PACKAGE_EXPORT = "package";

/** The indentation of a written manifest, as the workspace's formatter writes it. */
const MANIFEST_INDENTATION = 4;

/** A JavaScript identifier, which a property key can be without quotes. */
const IDENTIFIER = /^[A-Za-z_$][\w$]*$/u;

/** The fields of a manifest an edit reads: its root export and its dependencies. */
const Manifest = schema.looseObject({
    exports: schema.union([schema.string(), schema.looseObject({ ".": schema.string() })]),
    dependencies: schema.record(schema.string(), schema.string()).default({}),
});

/** A stack's installations object in a module, with the module's text and parsed form. */
interface StackDeclaration {
    /** The module's path, relative to the package directory. */
    readonly path: string;
    /** The module's text. */
    readonly text: string;
    /** The module's parsed form. */
    readonly program: Program;
    /** The `installations` object of the stack's definition. */
    readonly installations: ObjectExpression;
}

/** A package's source files as edits read and change them, by paths relative to its directory. */
export class PackageSource {
    /** Read a file of the package directory. */
    readonly #read: (path: string) => Promise<string>;
    /** Format a changed file's text as its workspace's formatter writes it. */
    readonly #format: (path: string, text: string) => Promise<string>;
    /** The changed files' texts, by path. */
    readonly #changed = new Map<string, string>();

    /** Edit a package directory's files through a reader and a formatter. */
    constructor(
        read: (path: string) => Promise<string>,
        format: (path: string, text: string) => Promise<string>,
    ) {
        this.#read = read;
        this.#format = format;
    }

    /** Apply one operation to the files, reading each file's latest text. */
    async apply(operation: EditOperation): Promise<void> {
        // change the manifest's dependencies
        if (operation.kind === "dependency.add" || operation.kind === "dependency.remove") {
            const text = await this.#text(MANIFEST);
            this.#changed.set(MANIFEST, PackageSource.#depend(text, operation));
        }
        // install a package in a stack
        else if (operation.kind === "installation.add") {
            const declaration = await this.#stack(operation.stack);
            this.#changed.set(declaration.path, PackageSource.#install(declaration, operation));
        }
        // remove an installation from a stack
        else {
            const declaration = await this.#stack(operation.stack);
            this.#changed.set(declaration.path, PackageSource.#uninstall(declaration, operation));
        }
    }

    /** Format the changed files and answer their texts, by path. */
    async changed(): Promise<ReadonlyMap<string, string>> {
        const formatted = new Map<string, string>();
        for (const [path, text] of this.#changed) {
            formatted.set(path, await this.#format(path, text));
        }

        return formatted;
    }

    /** Read a file's latest text: its change, else the file. */
    async #text(path: string): Promise<string> {
        return this.#changed.get(path) ?? (await this.#read(path));
    }

    /** Find the module declaring a stack export, following re-exports from the package's root. */
    async #stack(name: string): Promise<StackDeclaration> {
        // start at the root export's module
        const { exports } = Manifest.parse(JSON.parse(await this.#text(MANIFEST)));
        const root = normalize(typeof exports === "string" ? exports : exports["."]);
        const queued = new Set([root]);
        const pending = [root];

        // walk the re-exported modules from the root once each, until one declares the stack
        for (let path = pending.shift(); path !== undefined; path = pending.shift()) {
            const text = await this.#text(path);
            const program = parseAst(text, { lang: "ts" }, path);

            for (const statement of program.body) {
                // take the stack's own declaration
                const installations = PackageSource.#declared(statement, name);
                const reexported = PackageSource.#reexported(statement, name);
                if (installations !== undefined) {
                    return { path, text, program, installations };
                }
                // queue a relative module re-exporting it, once
                else if (reexported !== undefined) {
                    const next = normalize(join(dirname(path), reexported));
                    if (!queued.has(next)) {
                        queued.add(next);
                        pending.push(next);
                    }
                }
            }
        }

        // refuse a stack no module declares
        throw StackError.of("STACK_NOT_FOUND", `stack '${name}' not found`);
    }

    /** Add or remove a dependency of a manifest, keeping its dependencies sorted. */
    static #depend(
        text: string,
        operation: Extract<EditOperation, { readonly kind: `dependency.${string}` }>,
    ): string {
        // read the manifest's dependencies
        const manifest = schema.looseObject({}).parse(JSON.parse(text));
        const { dependencies } = Manifest.pick({ dependencies: true }).parse(manifest);
        const changed = new Map(Object.entries(dependencies));

        // add or change the range
        if (operation.kind === "dependency.add") {
            changed.set(operation.package, operation.range);
        }
        // remove a dependency the manifest has, else refuse it
        else if (!changed.delete(operation.package)) {
            throw StackError.of(
                "DEPENDENCY_NOT_FOUND",
                `dependency '${operation.package}' not found`,
            );
        }

        // write the dependencies sorted, as package managers write them
        const sorted = [...changed].toSorted(([left], [right]) => left.localeCompare(right));
        const written = { ...manifest, dependencies: Object.fromEntries(sorted) };

        return `${JSON.stringify(written, null, MANIFEST_INDENTATION)}\n`;
    }

    /** Read the `installations` object of `export const <name> = defineStack({ ... })`. */
    static #declared(statement: Directive | Statement, name: string): ObjectExpression | undefined {
        // match an exported const declaring the name with a defineStack call
        const declaration =
            statement.type === "ExportNamedDeclaration" ? statement.declaration : null;
        const declarator =
            declaration?.type === "VariableDeclaration"
                ? declaration.declarations.find(
                      (each) => each.id.type === "Identifier" && each.id.name === name,
                  )
                : undefined;
        const call = declarator?.init;
        if (
            call?.type !== "CallExpression" ||
            call.callee.type !== "Identifier" ||
            call.callee.name !== DEFINE_STACK
        ) {
            return undefined;
        }

        // read its installations object
        const [definition] = call.arguments;
        const property =
            definition?.type === "ObjectExpression"
                ? PackageSource.#property(definition, "installations")
                : undefined;
        if (property?.value.type !== "ObjectExpression") {
            throw StackError.of(
                "DEFINITION_INVALID",
                `stack '${name}' is invalid: it must declare an installations object`,
            );
        }

        return property.value;
    }

    /** Read the relative module a statement re-exports a name from, if it does. */
    static #reexported(statement: Directive | Statement, name: string): string | undefined {
        // match `export * from`
        if (statement.type === "ExportAllDeclaration" && statement.exported === null) {
            return PackageSource.#relative(statement.source.value);
        }
        // match `export { name } from`
        else if (
            statement.type === "ExportNamedDeclaration" &&
            statement.source !== null &&
            statement.specifiers.some(
                (specifier) =>
                    (specifier.exported.type === "Literal"
                        ? specifier.exported.value
                        : specifier.exported.name) === name,
            )
        ) {
            return PackageSource.#relative(statement.source.value);
        }

        return undefined;
    }

    /** Read a module specifier when it is relative. */
    static #relative(specifier: string): string | undefined {
        return specifier.startsWith(".") ? specifier : undefined;
    }

    /** Read a property of an object expression by its key. */
    static #property(object: ObjectExpression, key: string): ObjectProperty | undefined {
        return object.properties.find(
            (property): property is ObjectProperty =>
                property.type === "Property" &&
                ((property.key.type === "Identifier" && property.key.name === key) ||
                    (property.key.type === "Literal" && property.key.value === key)),
        );
    }

    /** Add an installation of a package to a stack's installations, importing the package. */
    static #install(
        declaration: StackDeclaration,
        operation: Extract<EditOperation, { readonly kind: "installation.add" }>,
    ): string {
        // refuse a name the stack already installs
        const { text, program, installations } = declaration;
        if (PackageSource.#property(installations, operation.name) !== undefined) {
            throw StackError.of(
                "INSTALLATION_EXISTS",
                `installation '${operation.name}' already exists`,
            );
        }

        // choose the package's import: the module's existing one, else a new one
        const edited = new MagicString(text);
        const specifier = `${operation.package}/${PACKAGE_EXPORT}`;
        const imports = program.body.filter(
            (statement): statement is ImportDeclaration => statement.type === "ImportDeclaration",
        );
        const existing = PackageSource.#imported(imports, specifier);
        const local = existing ?? PackageSource.#local(operation.package);
        const lastImport = imports.at(-1);

        // refuse a new name the module already uses
        if (existing === undefined && PackageSource.#names(program, []).has(local)) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `module '${declaration.path}' is invalid: '${local}' must be free to import '${specifier}'`,
            );
        }
        // import a new name first in a module without imports
        else if (existing === undefined && lastImport === undefined) {
            edited.prepend(`import ${local} from "${specifier}";\n`);
        }
        // import a new name after the last import
        else if (existing === undefined && lastImport !== undefined) {
            edited.appendLeft(lastImport.end, `\nimport ${local} from "${specifier}";`);
        }

        // append the installation after the last one and its comma
        const key = IDENTIFIER.test(operation.name)
            ? operation.name
            : JSON.stringify(operation.name);
        const name = JSON.stringify(operation.name);
        const entry = `${key}: ${INSTALL}(${local}, {}, { name: ${name} }),`;
        const lastEntry = installations.properties.at(-1);
        const comma = lastEntry === undefined ? "" : PackageSource.#comma(text, lastEntry);
        const after =
            lastEntry === undefined ? installations.start + 1 : lastEntry.end + comma.length;
        edited.appendLeft(after, `${lastEntry === undefined || comma !== "" ? "" : ","}\n${entry}`);

        return edited.toString();
    }

    /** Remove an installation from a stack's installations, and its import once unread. */
    static #uninstall(
        declaration: StackDeclaration,
        operation: Extract<EditOperation, { readonly kind: "installation.remove" }>,
    ): string {
        // refuse a name the stack does not install
        const { text, program, installations } = declaration;
        const property = PackageSource.#property(installations, operation.name);
        if (property === undefined) {
            throw StackError.of(
                "INSTALLATION_NOT_FOUND",
                `installation '${operation.name}' not found`,
            );
        }

        // remove the property with its comma
        const edited = new MagicString(text);
        edited.remove(property.start, property.end + PackageSource.#comma(text, property).length);

        // read the import of the installed package
        const [installed] =
            property.value.type === "CallExpression" ? property.value.arguments : [];
        const local = installed?.type === "Identifier" ? installed.name : undefined;
        const imported = program.body.find(
            (statement): statement is ImportDeclaration =>
                statement.type === "ImportDeclaration" &&
                statement.specifiers.length === 1 &&
                statement.specifiers[0]?.local.name === local,
        );

        // remove it once nothing else in the module reads it
        const isRead =
            local !== undefined && PackageSource.#names(program, [property, imported]).has(local);
        if (imported !== undefined && !isRead) {
            const newline = text[imported.end] === "\n" ? 1 : 0;
            edited.remove(imported.start, imported.end + newline);
        }

        return edited.toString();
    }

    /** Read the comma and the space before it that follow a node, if any. */
    static #comma(text: string, node: Span): string {
        return /^\s*,/u.exec(text.slice(node.end))?.[0] ?? "";
    }

    /** Collect the identifiers in a tree, leaving out some subtrees. */
    static #names(root: object, skipped: readonly (object | undefined)[]): Set<string> {
        // walk the tree depth first
        const names = new Set<string>();
        const pending: unknown[] = [root];
        for (let value = pending.pop(); value !== undefined; value = pending.pop()) {
            // descend into arrays
            if (Array.isArray(value)) {
                value.forEach((item: unknown) => pending.push(item));
            }
            // read a node outside the skipped subtrees, and descend into its fields
            else if (typeof value === "object" && value !== null && !skipped.includes(value)) {
                const name: unknown = Reflect.get(value, "name");
                if (Reflect.get(value, "type") === "Identifier" && typeof name === "string") {
                    names.add(name);
                }
                for (const key of Object.keys(value)) {
                    const field: unknown = Reflect.get(value, key);
                    pending.push(field);
                }
            }
        }

        return names;
    }

    /** Read the local name of a module's default import of a specifier, if any. */
    static #imported(imports: readonly ImportDeclaration[], specifier: string): string | undefined {
        return imports
            .filter((statement) => statement.source.value === specifier)
            .flatMap((statement) => statement.specifiers)
            .find((each) => each.type === "ImportDefaultSpecifier")?.local.name;
    }

    /** Choose the local name of a package's import: `@ada/task-list` as `taskListPackage`. */
    static #local(name: string): string {
        // join the unscoped name's words in camel case
        const unscoped = name.slice(name.lastIndexOf("/") + 1);
        const words = unscoped.split(/[^a-zA-Z0-9]+/u).filter((word) => word.length > 0);
        const camel = words
            .map((word, index) =>
                index === 0
                    ? word.toLowerCase()
                    : `${word.charAt(0).toUpperCase()}${word.slice(1)}`,
            )
            .join("");

        return `${camel}Package`;
    }
}

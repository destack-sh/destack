import { schema } from "@destack/schema";
import { Language } from "./language.ts";
import { Runtime } from "../runtime/index.ts";
import { TemplateDefinition } from "../template/index.ts";
import { DependencyName, PackageId, PackageName } from "./package.ts";
import {
    DeclarationConstructorMap,
    DeclarationKindDefinition,
    FunctionReference,
    StampMap,
} from "./constructor.ts";
import { Sandbox } from "./sandbox.ts";
import { PackagePath } from "../file/file.ts";
import { PackageError } from "../error/index.ts";

/** The export of a package's test layer, serving its dependents' tests outside any build. */
export const TEST_EXPORT = "./test";

/** The subpaths an app exports: its interface, its implementation and its test kit. */
const APP_SUBPATHS = new Set([
    "package",
    "object",
    "declare",
    "client",
    "stack",
    "setting",
    "view",
    "error",
    "server",
    "test",
    "inspect",
    "provider",
    "destack.json",
]);

/** A finding the package accepts in some files with a reason, failing once nothing matches. */
export const Expectation = schema.object({
    /** The files the findings are in, relative to the package. */
    files: schema.array(schema.string().min(1)).min(1),
    /** The rules whose findings are accepted, such as `eslint/no-console`. */
    rules: schema.array(schema.string().regex(/^[a-z][a-z0-9-]*\/[a-z][a-z0-9-]*$/u)).min(1),
    /** Why the findings are right here. */
    reason: schema.string().min(1),
});
/** A finding the package accepts in named files, with its reason. */
export type Expectation = schema.Infer<typeof Expectation>;

/** How checking treats the files of a package or workspace. */
const CheckSettings = schema.object({
    /** The accepted findings, each with its reason. */
    expect: schema.array(Expectation).min(1),
});

/** The JSON Schema address of the release that wrote a definition. */
const SchemaAddress = schema
    .string()
    .regex(/^https:\/\/destack\.app\/schemas\/[^/]+\/destack\.json$/u);

/** The settings a workspace root shares with its members. */
const WorkspaceSettings = schema.object({
    /** How checking treats the workspace's files: accepted findings, by root-relative paths. */
    check: CheckSettings.exactOptional(),
});

/** What a package is: part of the runtime, bundled by its importers, or installed. */
export const PackageRole = schema.enum(["builtin", "library", "app"]);
/** What a package is: part of the runtime, bundled by its importers, or installed. */
export type PackageRole = schema.Infer<typeof PackageRole>;

/** The declarations authored in destack.json. */
const definition = schema.object({
    /** The JSON Schema address of the release that wrote the definition. */
    $schema: SchemaAddress,
    /** The immutable identity assigned when creating this package. */
    id: PackageId,
    /** What the package is: a builtin, a library or an app. */
    role: PackageRole,
    /** The language used by the package. */
    language: Language,
    /** Source generation settings for a registry template package. */
    template: TemplateDefinition.exactOptional(),
    /** The runtimes the package's exports compile for. */
    runtimes: schema.array(Runtime).min(1).exactOptional(),
    /** The modules a host starts beside the bin, such as Worker scripts. */
    entrypoints: schema.array(PackagePath).min(1).exactOptional(),
    /** The packages the package's programs share with the code they load, beyond the builtins. */
    shares: schema.array(DependencyName).min(1).exactOptional(),
    /** What the package's workloads and views always use beyond the bare sandbox. */
    sandbox: Sandbox.exactOptional(),
    /** The declaration constructors the build describes, by name. */
    declarations: DeclarationConstructorMap.exactOptional(),
    /** The kinds the package defines for its dependents' constructors, such as `resource`. */
    kinds: schema.array(DeclarationKindDefinition).exactOptional(),
    /** The functions, tags and constructors receiving the calling module, by export path. */
    stamps: StampMap.exactOptional(),
    /** The extension building dependent packages, such as `./build#viewExtension`. */
    build: FunctionReference.exactOptional(),
    /** The packages whose build extensions compile this package beyond its runtime dependencies. */
    extensions: schema.array(PackageName).min(1).exactOptional(),
    /** How checking treats the package: the findings it accepts. */
    check: CheckSettings.exactOptional(),
    /** The settings of the workspace this package's directory roots. */
    workspace: WorkspaceSettings.exactOptional(),
    /** Runtime overrides keyed by the names in package.json exports. */
    exports: schema
        .record(
            schema.string(),
            schema.object({
                /** The runtimes this export compiles for, replacing the package's. */
                runtimes: schema.array(Runtime).min(1),
            }),
        )
        .exactOptional(),
});

/** The declarations authored in destack.json. */
export const PackageDefinition = Object.assign(definition, {
    /** Parse the text of a destack.json, refusing sandbox abilities its runtimes cannot honour. */
    read(text: string): PackageDefinition {
        return requireRuntimes(definition.parse(JSON.parse(text)));
    },
    /** Refuse the exports a package's role forbids. */
    requireExports(value: PackageDefinition, name: string, entries: readonly string[]): void {
        // refuse an app without its package
        const isPackaged = entries.some((entry) => subpathOf(entry) === "package");
        if (value.role === "app" && !isPackaged) {
            throw PackageError.of(
                "EXPORT_INVALID",
                `app '${name}' is invalid: it exports no ./package`,
            );
        }
        // refuse a library's package
        else if (value.role === "library" && isPackaged) {
            throw PackageError.of(
                "EXPORT_INVALID",
                `library '${name}' is invalid: it exports ./package`,
            );
        }

        // refuse an app's subpath outside the standard ones and its providers
        const free = entries.find((entry) => {
            const subpath = subpathOf(entry);

            return !APP_SUBPATHS.has(subpath) && !subpath.startsWith("provider/");
        });
        if (value.role === "app" && free !== undefined) {
            throw PackageError.of(
                "EXPORT_INVALID",
                `app '${name}' is invalid: it exports '${free}', outside the standard subpaths`,
            );
        }
    },
    /** Refuse importing an export of another package where the layout forbids it. */
    requireImport(
        importer: { readonly name: string; readonly role: PackageRole; readonly isTest: boolean },
        imported: { readonly name: string; readonly role: PackageRole },
        entry: string,
    ): void {
        // allow a package and tests every import
        if (importer.name === imported.name || importer.isTest) {
            return;
        }

        // read the subpath: an app's implementation is private to it, a test kit to tests
        const subpath = subpathOf(entry);
        const specifier = subpath === "" ? imported.name : `${imported.name}/${subpath}`;
        const isPrivate = subpath === "server" || subpath.startsWith("provider/");
        const isTestKit = subpath === "test" || subpath.startsWith("test/");

        // refuse an app's implementation
        if (imported.role === "app" && isPrivate) {
            throw PackageError.of(
                "DEPENDENCY_INVALID",
                `'${importer.name}' is invalid: it imports '${specifier}', private to its app`,
            );
        }
        // refuse a test kit outside tests
        else if (isTestKit) {
            throw PackageError.of(
                "DEPENDENCY_INVALID",
                `'${importer.name}' is invalid: it imports '${specifier}' outside its tests`,
            );
        }
        // refuse any app in a builtin, as the runtime never loads userland
        else if (importer.role === "builtin" && imported.role === "app") {
            throw PackageError.of(
                "DEPENDENCY_INVALID",
                `builtin '${importer.name}' is invalid: it imports app '${specifier}'`,
            );
        }
    },
    /** List the runtimes an export compiles for: its own, or else the package's. */
    runtimes(value: PackageDefinition, name: string): Runtime[] {
        const runtimes = value.exports?.[name]?.runtimes ?? value.runtimes;
        if (runtimes === undefined) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `export '${name}' is invalid: it declares no runtimes`,
            );
        }

        return runtimes;
    },
});

/** The validated contents of destack.json. */
export type PackageDefinition = schema.Infer<typeof definition>;

/** The destack.json of a workspace root that is not a package. */
const workspaceDefinition = schema.object({
    /** The JSON Schema address of the release that wrote the definition. */
    $schema: SchemaAddress,
    /** The settings the workspace shares with its members. */
    workspace: WorkspaceSettings,
});

/** The contents of a destack.json: a package's definition, or a workspace root's. */
const anyDefinition = schema.union([definition, workspaceDefinition]);

/** The contents of a destack.json: a package's definition, or a workspace root's. */
export const Definition = Object.assign(anyDefinition, {
    /** Parse a destack.json, refusing sandbox abilities its runtimes cannot honour. */
    read(text: string): Definition {
        const value = anyDefinition.parse(JSON.parse(text));

        return "id" in value ? requireRuntimes(value) : value;
    },
    /** Read the package a definition defines, nothing for a workspace root without a package. */
    package(value: Definition): PackageDefinition | undefined {
        return "id" in value ? value : undefined;
    },
});

/** The contents of a destack.json. */
export type Definition = schema.Infer<typeof anyDefinition>;

/** Read an export entry's subpath without its leading `./`, empty for the root. */
function subpathOf(entry: string): string {
    return entry.replace(/^\.\/?/u, "");
}

/** Refuse the process ability of a package compiling for no Bun runtime. */
function requireRuntimes(value: PackageDefinition): PackageDefinition {
    // run workloads in a Bun process only from a package compiling for Bun
    const runtimes = [
        ...(value.runtimes ?? []),
        ...Object.values(value.exports ?? {}).flatMap((entry) => entry.runtimes),
    ];
    if (value.sandbox?.process !== undefined && !runtimes.includes("bun")) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            "process ability is invalid: it requires the bun runtime",
        );
    }

    return value;
}

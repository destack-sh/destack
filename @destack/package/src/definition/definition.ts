import { defineSchema, schema } from "@destack/schema";
import { Language } from "./language.ts";
import { Runtime } from "../runtime/index.ts";
import { TemplateDefinition } from "../template/index.ts";
import { PackageId } from "./package.ts";
import { DeclarationConstructorMap, FunctionReference } from "./constructor.ts";
import { Publication } from "./publication.ts";
import { Capabilities } from "./capability.ts";
import { PackageError } from "../error/index.ts";

/** A finding the package accepts in named files, with its reason; checking fails once nothing matches it. */
export const Expectation = defineSchema(
    schema.object({
        /** The files the findings are in, relative to the package. */
        files: schema.array(schema.string().min(1)).min(1),
        /** The rules whose findings are accepted, such as `eslint/no-console`. */
        rules: schema.array(schema.string().regex(/^[a-z][a-z0-9-]*\/[a-z][a-z0-9-]*$/u)).min(1),
        /** Why the findings are right here. */
        reason: schema.string().min(1),
    }),
);
/** A finding the package accepts in named files, with its reason. */
export type Expectation = schema.Infer<typeof Expectation>;

/** How checking treats the files of a package or workspace. */
const CheckSettings = defineSchema(
    schema.object({
        /** The accepted findings, each with its reason. */
        expect: schema.array(Expectation).min(1),
    }),
);

/** The JSON Schema address of the release that wrote a definition. */
const SchemaAddress = schema
    .string()
    .regex(/^https:\/\/destack\.app\/schemas\/[^/]+\/destack\.json$/u);

/** The settings a workspace root shares with its members. */
const WorkspaceSettings = defineSchema(
    schema.object({
        /** How checking treats the workspace's files: the findings it accepts, by paths relative to the root. */
        check: CheckSettings.exactOptional(),
    }),
);

/** The declarations authored in destack.json. */
const definition = defineSchema(
    schema.object({
        /** The JSON Schema address of the release that wrote the definition. */
        $schema: SchemaAddress,
        /** The immutable identity assigned when creating this package. */
        id: PackageId,
        /** The language used by the package. */
        language: Language,
        /** Source generation settings for a registry template package. */
        template: TemplateDefinition.exactOptional(),
        /** The runtimes the package's exports compile for. */
        runtimes: schema.array(Runtime).min(1).exactOptional(),
        /** What the package's workloads and views may reach, each consented to at installation. */
        capabilities: Capabilities.exactOptional(),
        /** The declaration constructors the package exports, by name. */
        declarations: DeclarationConstructorMap.exactOptional(),
        /** The extension building the packages that use this one, such as `./build#viewExtension`. */
        build: FunctionReference.exactOptional(),
        /** How the registry publishes the package. */
        publication: Publication.schema.exactOptional(),
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
    }),
);

/** The declarations authored in destack.json. */
export const PackageDefinition = Object.assign(definition, {
    /** Parse the text of a destack.json, refusing capabilities its runtimes cannot honour. */
    read(text: string): PackageDefinition {
        return requireRuntimes(definition.parse(JSON.parse(text)));
    },
    /** List the runtimes an export compiles for: its own, or else the package's. */
    runtimes(value: PackageDefinition, name: string): Runtime[] {
        const runtimes = value.exports?.[name]?.runtimes ?? value.runtimes;
        if (runtimes === undefined) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `no runtimes declared for export: ${name}`,
            );
        }

        return runtimes;
    },
});

/** The validated contents of destack.json. */
export type PackageDefinition = schema.Infer<typeof definition>;

/** The destack.json of a workspace root that is no package. */
const workspaceDefinition = defineSchema(
    schema.object({
        /** The JSON Schema address of the release that wrote the definition. */
        $schema: SchemaAddress,
        /** The settings the workspace shares with its members. */
        workspace: WorkspaceSettings,
    }),
);

/** The contents of a destack.json: a package's definition, or a workspace root's. */
const anyDefinition = defineSchema(schema.union([definition, workspaceDefinition]));

/** The contents of a destack.json: a package's definition, or a workspace root's. */
export const Definition = Object.assign(anyDefinition, {
    /** Parse the text of a destack.json, refusing a package's capabilities its runtimes cannot honour. */
    read(text: string): Definition {
        const value = anyDefinition.parse(JSON.parse(text));

        return "id" in value ? requireRuntimes(value) : value;
    },
    /** Read the package a definition defines, nothing for a workspace root that is no package. */
    package(value: Definition): PackageDefinition | undefined {
        return "id" in value ? value : undefined;
    },
});

/** The contents of a destack.json. */
export type Definition = schema.Infer<typeof anyDefinition>;

/** Refuse the process capability of a package compiling for no Bun runtime. */
function requireRuntimes(value: PackageDefinition): PackageDefinition {
    // run workloads in a Bun process only from a package compiling for Bun
    const runtimes = [
        ...(value.runtimes ?? []),
        ...Object.values(value.exports ?? {}).flatMap((entry) => entry.runtimes),
    ];
    if (value.capabilities?.process !== undefined && !runtimes.includes("bun")) {
        throw new PackageError(
            "INVALID_DEFINITION",
            "the process capability requires the bun runtime",
        );
    }

    return value;
}

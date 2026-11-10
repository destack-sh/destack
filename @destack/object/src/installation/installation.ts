import { none, principal, self } from "@destack/access";
import { Expression, type Select, type Snapshot } from "@destack/db";
import { ComputeDefinition, DeclarationName, DirectoryGrant, PackageId } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { InstallationName } from "./name.ts";
import { InstallationBindings, InstallationSelection } from "./selection.ts";
import { field } from "../field/field.ts";
import { defineObject } from "../object/object.ts";
import type { PackageDescription } from "../declare/description.ts";

/** A package installed in a space at a selection, under a name. */
export const installation = defineObject(
    {
        name: "installation",
        plural: "installations",
        scope: [principal.space],
        represents: principal.installation,
        key: "name",
        // let its controller roll it out, apply its plans once approved, and finalize an uninstall
        controlled: { approval: true },
        // propagate each installation into the scopes inside its scope, read there by permission
        propagated: {},
        fields: {
            /** The installed package. */
            packageId: field.string(PackageId),
            /** What it follows: a release, a repository reference or a checkout. */
            selection: field.json(InstallationSelection),
            /** The revision its selection resolves to, absent before the first. */
            revisionId: field.string(schema.identifier("installation-revision")).optional(),
            /** The targets it selects for its package's declarations. */
            bindings: field.json(InstallationBindings).default({}),
            /** The stack its package exports for its space, absent for an application. */
            export: field.string().optional(),
            /** The arguments of the exported stack, absent for an application. */
            parameters: field.json(schema.jsonObject()).optional(),
            /** The space-local address. */
            name: field.string(InstallationName),
            /** Whether the installation should serve requests. */
            status: field.enum(["enabled", "suspended"]).default("enabled"),
            /** Workload compute settings checked against package and machine policy. */
            compute: field.json(schema.record(DeclarationName, ComputeDefinition)).default({}),
            /** The optional capabilities the scope's stack declares the installation allows. */
            capabilities: field.json(schema.array(DeclarationName)).default([]),
            /** The optional capabilities allowed at the installation's request, once approved. */
            allowed: field.json(schema.array(DeclarationName)).default([]),
            /** The directories granted to the package's `files` ability, each with its access. */
            directories: field.json(schema.array(DirectoryGrant)).default([]),
            /** How its server is isolated: a sandbox, or the machine's own process once trusted. */
            isolation: field.enum(["sandbox", "host"]).default("sandbox"),
        },
        // let permissions read the scope an installation lives in
        attributes: { scope: "string" },
        permissions: {
            read: none(),
            list: none(),
            install: none(),
            update: none(),
            delete: none(),
            replicate: self(),
            run: self(),
            observe: none(),
            analyze: none(),
            unmask: none(),
            allow: none(),
        },
        reserved: ["replicate", "run"],
        // bind roles on an installation directly
        shareable: { by: "update" },
        convert: {
            // read the process isolation that releases before 2026.11.0 stored as host
            "2026.11.0": {
                isolation: Expression.case(
                    Expression.column("isolation"),
                    [{ when: "process", then: Expression.literal("host") }],
                    Expression.column("isolation"),
                ),
            },
        },
        constraints: {
            managerInstallation: {
                kind: "foreignKey",
                on: ["scope", "managerInstallationId"],
                references: { object: "self", on: ["scope", "id"] },
            },
            scopePackage: { kind: "unique", on: ["scope", "packageId"] },
            scopeId: { kind: "unique", on: ["scope", "id"] },
            scopeIdPackage: { kind: "unique", on: ["scope", "id", "packageId"] },
            package: { kind: "index", on: ["packageId"] },
            // evaluate an exported stack with its arguments, one stack per space
            stack: {
                kind: "check",
                condition: {
                    OR: [
                        { export: { isNotNull: true }, parameters: { isNotNull: true } },
                        { export: { isNull: true }, parameters: { isNull: true } },
                    ],
                },
            },
            scopeStack: {
                kind: "index",
                on: ["scope"],
                unique: true,
                where: { export: { isNotNull: true } },
            },
        },
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("install", {
            fields: [
                "packageId",
                "name",
                "selection",
                "bindings",
                "export",
                "parameters",
                "status",
                "compute",
                "capabilities",
                "directories",
                "isolation",
            ],
        }),
        update: method.update("update", {
            fields: [
                "selection",
                "bindings",
                "export",
                "parameters",
                "status",
                "compute",
                "capabilities",
                "directories",
                "isolation",
            ],
        }),
        delete: method.delete("delete"),

        /** Target a revision its selection resolves to, which its controller rolls out. */
        target: method.mutation({
            permission: "control",
            input: schema.object({
                /** The revision to roll out. */
                revisionId: schema.identifier("installation-revision"),
            }),
        }),

        /** Allow one of the package's optional capabilities, as the installation asked for it. */
        allow: method.mutation({
            permission: "allow",
            input: schema.object({ capability: DeclarationName }),
        }),
        /** Withdraw an optional capability allowed at the installation's request. */
        disallow: method.mutation({
            permission: "allow",
            input: schema.object({ capability: DeclarationName }),
        }),
    }),
);

/** A persisted installation record. */
export type Installation = Select<typeof installation.table>;

/** The installations of a space. */
export const Installation = {
    /** Read which of some installations are applications: the ones without a stack's export. */
    async applications(
        reader: Pick<Snapshot, "read">,
        ids: readonly Identifier<"installation">[],
    ): Promise<ReadonlySet<Identifier<"installation">>> {
        const rows = await reader.read(installation.table).findMany({
            where: { id: { in: [...new Set(ids)] }, export: { isNull: true } },
        });

        return new Set(rows.map((row) => row.id));
    },

    /** List the packages only a package's optional capabilities need installed. */
    optionalPackages(description: PackageDescription | undefined): PackageId[] {
        return Object.values(description?.capabilities ?? {}).flatMap((capability) =>
            capability.optional === true ? (capability.packages ?? []) : [],
        );
    },
};

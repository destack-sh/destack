import {
    type Authorization,
    InstallationId,
    none,
    principal,
    resource,
    Role,
    self,
} from "@destack/access";
import { Expression, type Select, type Snapshot, Table } from "@destack/db";
import { ComputeDefinition, DeclarationName, DirectoryGrant, PackageId } from "@destack/package";
import { type DigestCrypto, type Identifier, schema } from "@destack/schema";
import { ObjectReference, Scope } from "@destack/sync";
import { InstallationName } from "../declare/egress.ts";
import { InstallationBindings, InstallationSelection } from "./selection.ts";
import { field } from "../field/field.ts";
import { defineObject } from "../object/object.ts";
import type { ObjectPackage } from "../server/implementation.ts";
import type { BaseServerContext } from "../workload/workload.ts";
import type { PackageDescription } from "../declare/description.ts";

/** A package installed in a partition scope: born with it, or installed at a selection. */
export const installation = defineObject(
    {
        name: "installation",
        plural: "installations",
        scope: [principal.space, principal.machine, principal.device, "universe"],
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
            /** The scope-local address, absent for a base installation, reached by its package. */
            name: field.string(InstallationName).optional(),
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
            // let every scope list the universe's installations, the platform services it runs on
            list: resource({ scope: Scope.universe.id }),
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
            // keep a base installation enabled, as its scope bore it
            base: {
                kind: "check",
                condition: { OR: [{ name: { isNotNull: true } }, { status: "enabled" }] },
            },
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

/** The installations of a scope: born with it, or installed through a rollout. */
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

    /** Receive a scope with its base installations, each born once and bound to its role. */
    async receive(
        authorization: Authorization,
        scope: ObjectReference,
        ancestors: readonly string[],
        packages: readonly ObjectPackage[],
    ): Promise<BaseServerContext[]> {
        // write the scope's row once, which the universe has without storing it
        const { transaction, environment } = authorization;
        const [received] = await Scope.read(transaction.view(), [scope.id]);
        if (received === undefined) {
            await authorization.receive(scope, ancestors);
        }
        const { subject: author, now } = authorization.context();

        // write each package's base installation once, binding it to its role each time
        const contexts: BaseServerContext[] = [];
        for (const handle of packages) {
            // name the installation by its scope and package, its row authored as the scope's receiver
            const based = await Installation.base(scope, handle, environment.crypto);
            const id = based.installation;

            // write the installation once the scope lacks it
            const born = await transaction.read(installation.table).findFirst({ where: { id } });
            if (born === undefined) {
                const row = Table.complete(installation.table, {
                    id,
                    scope: scope.id,
                    packageId: handle.id,
                    selection: { kind: "release", version: handle.version },
                    createdAt: now,
                    updatedAt: now,
                    ...ObjectReference.columns("createdBy", author),
                    ...ObjectReference.columns("updatedBy", author),
                });
                const change = { table: installation.table, before: null, after: row };
                await transaction.write(change);
            }

            // bind the installation to its package's role in the scope
            const subject = principal.installation.reference(scope.id, id);
            await Role.applyInherent(
                scope,
                Installation.role(scope, handle),
                subject,
                authorization,
            );
            contexts.push(based);
        }

        return contexts;
    },

    /** Describe a package's base installation in a scope, identified by the scope and package. */
    async base(
        scope: ObjectReference,
        handle: Pick<ObjectPackage, "id" | "name" | "capabilities">,
        crypto: DigestCrypto,
    ): Promise<BaseServerContext> {
        // identify it by its scope and package, which reach it in place of a name, running its required capabilities
        const installed = await InstallationId.base(scope, handle.id, crypto);
        const capabilities = Object.entries(handle.capabilities)
            .filter(([, capability]) => capability.optional !== true)
            .map(([name]) => name);

        return { installation: installed, scope: scope.id, capabilities };
    },

    /** Describe a base installation's role: what its package requests in its scope. */
    role(scope: ObjectReference, handle: ObjectPackage) {
        // take what the package's installations request in their scope, its own types going by its policies
        const requested = handle.permissions.space ?? {};
        const permissions =
            scope.id === Scope.universe.id ? (requested.universe ?? []) : (requested.space ?? []);

        return {
            name: `base:${handle.name}`,
            description: `What ${handle.name} does in its scope, as it was born with it`,
            permissions,
        };
    },

    /** List the packages only a package's optional capabilities need installed. */
    optionalPackages(description: PackageDescription | undefined): PackageId[] {
        return Object.values(description?.capabilities ?? {}).flatMap((capability) =>
            capability.optional === true ? (capability.packages ?? []) : [],
        );
    },
};

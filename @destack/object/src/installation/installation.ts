import { type Authorization, InstallationId, none, principal, Role, self } from "@destack/access";
import { type Select, Table } from "@destack/db";
import { ComputeDefinition, DeclarationName, DirectoryGrant, PackageId } from "@destack/package";
import { type DigestCrypto, Instant, schema } from "@destack/schema";
import { ObjectReference, Scope } from "@destack/sync";
import { InstallationName } from "../declare/egress.ts";
import { InstallationBindings, InstallationSelection } from "./selection.ts";
import { field } from "../field/field.ts";
import { defineObject } from "../object/object.ts";
import type { ObjectPackage } from "../server/implementation.ts";
import type { BaseServerContext } from "../workload/workload.ts";
import type { PackageDescription } from "../declare/description.ts";

/** A short-lived access token a scope grants one of its installations for a package. */
export const InstallationToken = schema.object({
    /** The signed access token. */
    accessToken: schema.string(),
    /** The HTTP authorization scheme. */
    tokenType: schema.literal("Bearer"),
    /** The exclusive expiry in epoch milliseconds. */
    expiresAt: Instant,
});
/** A short-lived access token a scope grants one of its installations. */
export type InstallationToken = schema.Infer<typeof InstallationToken>;

/** A package installed in a partition scope: born with it, or installed at a selection. */
export const installation = defineObject(
    {
        name: "installation",
        plural: "installations",
        scope: [principal.space, principal.machine, principal.device, "universe"],
        represents: principal.installation,
        // let its controller roll it out, apply its plans once approved, and finalize an uninstall
        controlled: { approval: true },
        // hand the universe's installations down to every scope, as every space's chain includes them
        inherited: { where: { scope: Scope.universe.id } },
        fields: {
            /** The installed package. */
            packageId: field.string(PackageId),
            /** What it follows: a release, a repository reference or a checkout. */
            selection: field.json(InstallationSelection),
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
        },
        permissions: {
            read: none(),
            list: none(),
            install: none(),
            update: none(),
            delete: none(),
            replicate: self(),
            run: self(),
            token: self(),
            observe: none(),
            analyze: none(),
            unmask: none(),
            allow: none(),
        },
        reserved: ["replicate", "run", "token"],
        // bind roles on an installation directly
        shareable: { by: "update" },
        constraints: {
            managerInstallation: {
                kind: "foreignKey",
                on: ["scope", "managerInstallationId"],
                references: { object: "self", on: ["scope", "id"] },
            },
            scopeName: { kind: "unique", on: ["scope", "name"] },
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
            ],
        }),
        delete: method.delete("delete"),

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

        /** Grant the installation a token of its scope, as Kubernetes' TokenRequest. */
        token: method.mutation({
            permission: "token",
            input: schema.object({
                /** The receiving package. */
                audience: PackageId,
            }),
            output: InstallationToken,
        }),
    }),
);

/** A persisted installation record. */
export type Installation = Select<typeof installation.table>;

/** The installations of a scope: born with it, or installed through a rollout. */
export const Installation = {
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
                installation.updateReaders(transaction, change, now);
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
        const requested = handle.permissions.installation ?? {};
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

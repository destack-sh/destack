import {
    type Authorization,
    InstallationId,
    none,
    PermissionReference,
    principal,
    Role,
    self,
} from "@destack/access";
import { type Select, Table } from "@destack/db";
import { ComputeDefinition, DeclarationName, DirectoryGrant, PackageId } from "@destack/package";
import { type DigestCrypto, type Identifier, Instant, schema } from "@destack/schema";
import { ObjectReference, Scope } from "@destack/sync";
import { InstallationName } from "../declare/egress.ts";
import { field } from "../field/field.ts";
import type { MethodContext } from "../method/context.ts";
import { defineObject, type ObjectType } from "../object/object.ts";
import type { ObjectPackage } from "../server/implementation.ts";
import type { ObjectStore } from "../server/store.ts";
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

/** A package installed in a partition scope: born with it, declared by its stack, or installed. */
export const installation = defineObject(
    {
        name: "installation",
        plural: "installations",
        scope: [principal.space, principal.machine, principal.device, "universe"],
        represents: principal.installation,
        // hand the universe's installations down to every scope, as every space's chain includes them
        inherited: { where: { scope: Scope.universe.id } },
        fields: {
            /** Whether its scope bore it, its stack declares it, or someone installed it. */
            origin: field.enum(["base", "declared", "installed"]),
            /** The installed package. */
            packageId: field.string(PackageId),
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
                condition: { OR: [{ origin: { ne: "base" } }, { status: "enabled" }] },
            },
            // name every installation but a base one, which its package identifies
            named: {
                kind: "check",
                condition: {
                    OR: [
                        { origin: "base", name: { isNull: true } },
                        { origin: { ne: "base" }, name: { isNotNull: true } },
                    ],
                },
            },
        },
    },
    (method) => ({
        list: method.list("list"),
        install: method.create("install", {
            isInternal: true,
            fields: [
                "origin",
                "packageId",
                "name",
                "status",
                "compute",
                "capabilities",
                "directories",
            ],
        }),
        update: method.update("update", {
            isInternal: true,
            fields: ["status", "compute", "capabilities", "directories"],
        }),
        delete: method.delete("delete", { isInternal: true }),

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
    /** The fields an install sets: package, name, status, compute, capabilities and directories. */
    fields: schema.object({
        packageId: PackageId,
        name: InstallationName,
        status: schema.enum(["enabled", "suspended"]).exactOptional(),
        compute: schema.record(DeclarationName, ComputeDefinition).exactOptional(),
        capabilities: schema.array(DeclarationName).exactOptional(),
        directories: schema.array(DirectoryGrant).exactOptional(),
    }),

    /** Receive a scope with its base installations, each born once and bound to its role. */
    async receive(
        authorization: Authorization,
        scope: ObjectReference,
        ancestors: readonly string[],
        packages: readonly ObjectPackage[],
        store: Pick<ObjectStore, "objects" | "readServer">,
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
                    origin: "base",
                    packageId: handle.id,
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
                Installation.role(
                    scope,
                    handle,
                    store.objects.filter((type) => store.readServer(type)?.installation === id),
                ),
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

        return { origin: "base", installation: installed, scope: scope.id, capabilities };
    },

    /** Describe a base installation's role: its served types' permissions and its requests. */
    role(scope: ObjectReference, handle: ObjectPackage, objects: readonly ObjectType[]) {
        // take every unreserved permission of the types it serves, which its code acts on
        const owned = objects.flatMap((type) => {
            const reserved = new Set<string>(type.definition.reserved ?? []);

            return type.permissions
                .filter((name) => !reserved.has(name))
                .map((name) => type.policy.permission(name));
        });

        // add what the package's installations request in their scope
        const requested = handle.permissions.installation ?? {};
        const asked =
            scope.id === Scope.universe.id ? (requested.universe ?? []) : (requested.space ?? []);

        // keep each permission once, which a package may both own and request
        const permissions = [...owned, ...asked].filter(
            (permission, index, all) =>
                all.findIndex(
                    (other) =>
                        PermissionReference.key(other) === PermissionReference.key(permission),
                ) === index,
        );

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

    /** Install a package in a scope as the calling code, in its call's transaction. */
    async install(
        context: MethodContext,
        scope: string,
        fields: {
            readonly origin: "declared" | "installed";
            readonly packageId: Identifier<"package">;
            readonly name: string;
            readonly status?: "enabled" | "suspended";
            readonly compute?: Installation["compute"];
            readonly capabilities?: string[];
            readonly directories?: Installation["directories"];
        },
    ): Promise<Identifier<"installation">> {
        // identify a declared installation by its scope and name, as its stack's readers derive it
        const declared =
            fields.origin === "declared"
                ? {
                      id: await InstallationId.declared(
                          { id: scope },
                          fields.name,
                          context.environment.crypto,
                      ),
                  }
                : {};
        const [created] = await context.mutate(context.self, [
            installation.call("install", { ...installation.in(scope), ...declared, ...fields }),
        ]);

        return created.id;
    },
};

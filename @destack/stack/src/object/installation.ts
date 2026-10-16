import { Catalog } from "@destack/locale";
import { none, PermissionReference, principal, self } from "@destack/access";
import { InstallationName } from "@destack/runtime";
import {
    and,
    type DatabaseConnection,
    eq,
    isNull,
    Snapshot,
    type Select,
    type SQL,
    type Transaction,
} from "@destack/db";
import { type ObjectOf, defineObject, field, ObjectError } from "@destack/object";
import type { Stack } from "@destack/object/server";
import {
    CapabilityName,
    ComputeDefinition,
    DeclarationName,
    DirectoryGrant,
    PackageId,
} from "@destack/package";
import { type BuildReader, ViewDescription } from "@destack/package/manifest";
import { Plan } from "@destack/resource";
import {
    defineSchema,
    Digest,
    type Identifier,
    Instant,
    type JsonValue,
    schema,
} from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { SettingCatalog, SettingDescription, SettingReference } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import { Scope } from "@destack/sync";
import {
    InstallationBuild,
    InstallationSelection,
    InstallationDefinition,
} from "../declare/installation.ts";
import { StackDefinition } from "../declare/stack.ts";
import { space, Space } from "@destack/account/object";
import { Submission } from "./stack.ts";

/** The account owning the repository of a written selection, null for any other selection. */
export const SelectedAccount = schema.identifier("account").nullable();

/** A view a build's output mounts. */
export const BuildView = ViewDescription.extend({
    /** The output mounting the view. */
    output: DeclarationName,
});
/** A view a build's output mounts. */
export type BuildView = schema.Infer<typeof BuildView>;

/** A location within a view, with its query, excluding another origin. */
export const ViewPath = schema.string().regex(/^\/(?:[^/\\\r\n][^\\\r\n]*)?$/u);

/** A view of an installation's revision, as a client opens it. */
export const InstallationView = schema.object({
    /** The installation. */
    installationId: schema.identifier("installation"),
    /** The revision whose build serves the view. */
    revisionId: schema.identifier("installation-revision"),
    /** The build serving the view. */
    build: InstallationBuild,
    /** The view's name. */
    view: DeclarationName,
    /** The view as the build's output mounts it. */
    definition: BuildView,
    /** The location within the view. */
    path: ViewPath,
    /** The opening person's home space, where the view's home types open. */
    home: schema.identifier("space"),
});
/** A view of an installation's revision, as a client opens it. */
export type InstallationView = schema.Infer<typeof InstallationView>;

/** A short-lived access token a space grants one of its installations for a package. */
export const InstallationToken = defineSchema(
    schema.object({
        /** The signed access token. */
        accessToken: schema.string(),
        /** The HTTP authorization scheme. */
        tokenType: schema.literal("Bearer"),
        /** The exclusive expiry in epoch milliseconds. */
        expiresAt: Instant,
    }),
);
/** A short-lived access token a space grants one of its installations. */
export type InstallationToken = schema.Infer<typeof InstallationToken>;

/** A package installed into a space: its stack or one of its applications. */
export const installation = defineObject(
    {
        name: "installation",
        plural: "installations",
        scope: space,
        represents: principal.installation,
        controlled: { approval: true },
        declarable: { schema: InstallationDefinition },
        fields: {
            /** The installed package. */
            packageId: field.string(PackageId),
            /** Whether the installation declares the space's contents or serves an application. */
            role: field.enum(["stack", "application"]).default("application"),
            /** The release, repository reference or checkout the installation follows. */
            selection: field.json(InstallationSelection),
            /** The exported stack definition or parameterised function, for stacks. */
            export: field.string().optional(),
            /** Arguments supplied to the exported definition function, for stacks. */
            parameters: field.json(schema.jsonObject()).optional(),
            /** The revision to apply, absent before the first submission. */
            revision: field.reference(() => installationRevision).optional(),
            /** The principal submitting the followed revision, absent before the first one. */
            submittedBy: field.subject().optional(),
            /** The digest of the space's stack the installation last submitted, absent for none. */
            spaceStack: field.string(Digest).optional(),
            /** The revision fully applied, absent before the first application. */
            appliedRevision: field.reference(() => installationRevision).optional(),
            /** The revision applied before the applied one, which a rollback follows again. */
            previousRevision: field.reference(() => installationRevision).optional(),
            /** Whether the installation should serve requests. */
            status: field.enum(["enabled", "suspended"]).default("enabled"),
            /** The space-local address for this installation. */
            name: field.string(InstallationName),
            /** Workload compute settings checked against package and machine policy. */
            compute: field.json(schema.record(DeclarationName, ComputeDefinition)).default({}),
            /** The optional capabilities the installation allows. */
            capabilities: field.json(schema.array(CapabilityName)).default([]),
            /** The directories granted to the package's `fs` capability, each with its access. */
            directories: field.json(schema.array(DirectoryGrant)).default([]),
            /** The digest of the objects the installation last related as their consumer. */
            consumption: field.string(Digest).optional(),
        },
        permissions: {
            read: none(),
            list: none(),
            create: none(),
            update: none(),
            delete: none(),
            apply: none(),
            open: none(),
            replicate: self(),
            run: self(),
            token: self(),
            follow: none(),
            deploy: none(),
            observe: none(),
            analyze: none(),
            unmask: none(),
        },
        reserved: ["replicate", "run", "token"],
        // bind roles on an installation directly
        shareable: { by: "update" },
        constraints: {
            space: {
                kind: "foreignKey",
                on: ["scope"],
                references: { object: () => space, on: ["id"] },
                onDelete: "no action",
            },
            managerInstallation: {
                kind: "foreignKey",
                on: ["scope", "managerInstallationId"],
                references: { object: "self", on: ["scope", "id"] },
            },
            scopeName: { kind: "unique", on: ["scope", "name"] },
            scopePackage: { kind: "unique", on: ["scope", "packageId"] },
            scopeId: { kind: "unique", on: ["scope", "id"] },
            spacePackage: { kind: "unique", on: ["scope", "id", "packageId"] },
            stack: { kind: "index", on: ["scope"], unique: true, where: { role: "stack" } },
            package: { kind: "index", on: ["packageId"] },
            revisionInstallation: {
                kind: "foreignKey",
                on: ["id", "revisionId"],
                references: { object: () => installationRevision, on: ["installationId", "id"] },
                onDelete: "no action",
            },
            appliedRevisionInstallation: {
                kind: "foreignKey",
                on: ["id", "appliedRevisionId"],
                references: { object: () => installationRevision, on: ["installationId", "id"] },
                onDelete: "no action",
            },
            previousRevisionInstallation: {
                kind: "foreignKey",
                on: ["id", "previousRevisionId"],
                references: { object: () => installationRevision, on: ["installationId", "id"] },
                onDelete: "no action",
            },
            stackExport: {
                kind: "check",
                condition: {
                    OR: [
                        {
                            role: "stack",
                            export: { isNotNull: true },
                            parameters: { isNotNull: true },
                        },
                        {
                            role: { ne: "stack" },
                            OR: [{ export: { isNull: true } }, { parameters: { isNull: true } }],
                        },
                    ],
                },
            },
        },
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("create", {
            fields: ["packageId", "selection", "name", "capabilities", "directories"],
            variants: { role: { stack: ["export", "parameters"], application: [] } },
            prepared: SelectedAccount,
        }),
        update: method.update("update", {
            fields: ["selection", "status", "capabilities", "directories"],
            prepared: SelectedAccount,
        }),
        delete: method.delete("delete"),

        /** Read the plan waiting for approval. */
        plan: method.query({ permission: "read", output: Plan }),

        /** Follow a submitted build. */
        submit: method.mutation({
            permission: "apply",
            input: Submission,
            prepared: SelectedAccount,
        }),

        /** Approve the plan waiting for approval by its digest. */
        approve: method.mutation({
            permission: "apply",
            input: schema.object({
                /** The digest of the reviewed plan. */
                plan: Digest,
            }),
            output: Plan,
        }),

        /** Read the build and view a client opens at a location, in the opening person's home. */
        open: method.query({
            permission: "open",
            isAudited: true,
            input: schema.object({
                /** The view the revision declares. */
                view: DeclarationName,
                /** The location within the view. */
                path: ViewPath,
            }),
            output: InstallationView,
        }),

        /** Follow the revision applied before the current one again. */
        rollBack: method.mutation({
            permission: "apply",
            output: schema.object({
                /** The revision the installation follows again. */
                revisionId: schema.identifier("installation-revision"),
            }),
        }),

        /** Deploy the revision an application follows, and drain a suspended one. */
        deploy: method.mutation({
            permission: "deploy",
            isInternal: true,
            output: schema.object({
                /** The permissions the deployed workloads request. */
                requested: schema
                    .object({
                        /** The permissions requested in the submitter's home. */
                        home: schema.array(PermissionReference),
                        /** The permissions requested in the space's account. */
                        account: schema.array(PermissionReference),
                    })
                    .nullable(),
            }),
        }),

        /** Grant the installation a token of its space, as Kubernetes' TokenRequest. */
        token: method.mutation({
            permission: "token",
            input: schema.object({
                /** The receiving package. */
                audience: PackageId,
            }),
            output: InstallationToken,
        }),

        /** Point an installation at the revision it follows, as its controller does. */
        follow: method.mutation({
            permission: "follow",
            isInternal: true,
            input: schema.object({
                /** The release, repository reference or checkout the revision evaluated. */
                selection: InstallationSelection.exactOptional(),
                /** The exported definition or parameterised function, for stacks. */
                export: schema.string().min(1).exactOptional(),
                /** Arguments for a parameterised definition, for stacks. */
                parameters: schema.jsonObject().exactOptional(),
                /** The revision to follow. */
                revisionId: schema.identifier("installation-revision"),
            }),
        }),
    }),
).implement({
    methods: {
        rollBack: async (call) => {
            // read the revision applied before the applied one
            const { previousRevisionId } = call.target;
            const previous =
                previousRevisionId === null
                    ? undefined
                    : await call
                          .read(installationRevision)
                          .findFirst({ where: { id: previousRevisionId } });
            // refuse an installation without one
            if (previous === undefined) {
                throw new CallError("CONFLICT", {
                    message: `installation ${call.target.name} has no revision before its applied one`,
                });
            }
            // refuse a revision of a build, whose repository or checkout the installation follows on
            else if (previous.build.kind !== "release") {
                throw new CallError("CONFLICT", {
                    message: `installation ${call.target.name} rolls back only to a release, its previous revision is a build`,
                });
            }

            // follow it and pin its release
            await call.update({
                revisionId: previous.id,
                selection: { kind: "release", version: previous.build.version },
            });

            return { revisionId: previous.id };
        },
        follow: (call) => call.update(call.input),
    },
});

/** An immutable evaluation of an installation's selected build. */
export const installationRevision = defineObject(
    {
        name: "installation-revision",
        plural: "installationRevisions",
        scope: space,
        fields: {
            /** The evaluated installation. */
            installation: field.reference(
                (): ObjectOf<{ identity: "installation" }> => installation,
                {
                    delete: "cascade",
                },
            ),
            /** The exact build evaluated. */
            build: field.json(InstallationBuild),
            /** The views the build's outputs mount, keyed by name. */
            views: field.json(schema.record(DeclarationName, BuildView)),
            /** The settings the build declares. */
            settings: field.json(schema.array(SettingDescription)),
            /** The translations of its own and its unpublished dependencies' messages. */
            catalogs: field.json(schema.array(Catalog)),
            /** The evaluated stack definition, for stacks. */
            definition: field.json(StackDefinition).optional(),
            /** The SHA-256 digest of the canonical build, parameters and definition. */
            digest: field.string(Digest),
        },
        permissions: ["read", "record"],
        constraints: {
            installationId: { kind: "unique", on: ["installationId", "id"] },
            digest: { kind: "unique", on: ["installationId", "digest"] },
            digestForm: {
                kind: "check",
                condition: { digest: { length: 64, characters: "0-9a-f" } },
            },
        },
    },
    (method) => ({
        create: method.create("record", {
            isInternal: true,
            fields: ["installationId", "build", "definition", "digest"],
            input: schema.object({
                /** The package whose build the revision evaluates. */
                packageId: PackageId,
            }),
        }),
    }),
);

/** A persisted installation record. */
export type Installation = Select<typeof installation.table>;

/** Reads of installations that other packages resolve references by. */
export const Installation = {
    /** Resolve an installation's value of each space setting its build declares, by setting key. */
    async settings(
        database: DatabaseConnection,
        build: BuildReader,
        target: {
            readonly scope: Identifier<"space">;
            readonly installation: Identifier<"installation">;
        },
    ): Promise<Record<string, JsonValue>> {
        // resolve each space setting the build declares for the installation
        const catalog = await SettingCatalog.read(build);
        const placed = catalog.settings.filter((declared) => declared.definition.scope === "space");
        const resolved = await Promise.all(
            placed.map(async (declared): Promise<[string, JsonValue]> => [
                SettingReference.key(declared.reference),
                schema
                    .json()
                    .parse(await SettingValue.resolve(Snapshot.live(database), declared, target)),
            ]),
        );

        return Object.fromEntries(resolved);
    },

    /** Require an installation of a space by identifier, rejecting the declaration otherwise. */
    async require(
        transaction: Transaction,
        spaceId: Identifier<"space">,
        id: Identifier<"installation">,
    ): Promise<Identifier<"installation">> {
        const [existing] = await transaction
            .read(installation.table)
            .findMany({ where: { scope: spaceId, id } });
        if (existing === undefined) {
            throw new ObjectError(
                "INVALID_DECLARATION",
                `installation ${id} is no installation of the space`,
            );
        }

        return existing.id;
    },

    /** Resolve a declared installation: one the stack declares by name, or the space's by id. */
    async resolve(
        reference: string | { readonly id: string },
        stack: Stack,
    ): Promise<Identifier<"installation">> {
        // resolve an installation the stack declares
        if (typeof reference === "string") {
            return schema
                .identifier("installation")
                .parse(await stack.require(installation, reference));
        }

        // require an existing installation of the space
        return Installation.require(
            stack.transaction,
            schema.identifier("space").parse(stack.scope),
            schema.identifier("installation").parse(reference.id),
        );
    },

    /** Read the fields recording a revision applied, keeping the one applied before. */
    applied(
        target: Pick<Installation, "appliedRevisionId" | "previousRevisionId">,
        revisionId: Identifier<"installation-revision">,
    ): Pick<Installation, "appliedRevisionId" | "previousRevisionId"> {
        const isSame = target.appliedRevisionId === revisionId;

        return {
            appliedRevisionId: revisionId,
            previousRevisionId: isSame ? target.previousRevisionId : target.appliedRevisionId,
        };
    },

    /** Read an installation a machine serves now. */
    async findServed(
        database: DatabaseConnection,
        id: Identifier<"installation">,
    ): Promise<Installation | undefined> {
        const [joined] = await database
            .select({ installation: installation.table })
            .from(installation.table)
            .innerJoin(space.table, eq(space.table.id, installation.table.scope))
            .where(and(eq(installation.table.id, id), Space.served(), launchable()));

        return joined?.installation;
    },

    /** Read where an installation's space moves while a transfer fences it here. */
    async moved(
        database: DatabaseConnection,
        id: Identifier<"installation">,
    ): Promise<{ readonly scope: string; readonly machine: string } | undefined> {
        // read the installation's space
        const [found] = await database
            .select({ scope: installation.table.scope })
            .from(installation.table)
            .where(eq(installation.table.id, id));
        if (found === undefined) {
            return undefined;
        }

        // find the fenced scope of its chain and the machine it moves to
        const chain = await Scope.chain(Snapshot.live(database), found.scope);
        const fenced = Scope.fenced(chain);

        return fenced?.movedTo === undefined
            ? undefined
            : { scope: fenced.object.id, machine: fenced.movedTo };
    },

    /** Read an installation a machine serves in a call's space by its identifier or name. */
    async requireServed(
        database: DatabaseConnection,
        installed: { readonly spaceId: string; readonly installation: string },
    ): Promise<Installation> {
        // take the installation served in the call's space, by its identifier or its name
        const id = await identify(database, installed);
        const served = id === undefined ? undefined : await Installation.findServed(database, id);
        if (served?.scope === installed.spaceId) {
            return served;
        }

        // answer a moving space's installation with where it moves
        const moved =
            served === undefined && id !== undefined
                ? await Installation.moved(database, id)
                : undefined;
        // send a moving space's caller to where it moves
        if (moved !== undefined) {
            throw new CallError("MISDIRECTED_REQUEST", {
                message: `${moved.scope} moves to ${moved.machine}`,
                data: moved,
            });
        }
        // let a caller wait for an installation not served yet, such as one still deploying
        else if (id !== undefined && served === undefined) {
            throw new CallError("UNAVAILABLE", {
                message: `installation ${installed.installation} in ${installed.spaceId} is not served yet`,
            });
        }
        // refuse an installation this machine does not have
        else {
            throw new CallError("NOT_FOUND", {
                message: `this machine serves no installation ${installed.installation} in ${installed.spaceId}`,
            });
        }
    },

    /** Find a space's enabled installation not being deleted by its name. */
    async find(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
        name: string,
    ): Promise<Installation | undefined> {
        const [found] = await database
            .select()
            .from(installation.table)
            .where(
                and(installation.inScope(spaceId), eq(installation.table.name, name), launchable()),
            );

        return found;
    },

    /** List a space's enabled installations not being deleted with the revisions they follow. */
    async list(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
    ): Promise<{ installation: Installation; revision: InstallationRevision }[]> {
        return database
            .select({ installation: installation.table, revision: installationRevision.table })
            .from(installation.table)
            .innerJoin(
                installationRevision.table,
                eq(installationRevision.table.id, installation.table.revisionId),
            )
            .where(and(installation.inScope(spaceId), launchable()));
    },
};

/** An immutable evaluation applied to an installation. */
export type InstallationRevision = Select<typeof installationRevision.table>;

/** The revisions of an installation: one per distinct build and stack evaluation. */
export const InstallationRevision = {
    /** Read the revision an installation follows, absent before its first. */
    async followed(
        database: DatabaseConnection,
        target: Pick<Installation, "revisionId">,
    ): Promise<InstallationRevision | undefined> {
        if (target.revisionId === null) {
            return undefined;
        }
        const [revision] = await database
            .select()
            .from(installationRevision.table)
            .where(eq(installationRevision.table.id, target.revisionId));

        return revision;
    },

    /** Digest a build and its stack evaluation. */
    digest(build: InstallationBuild, evaluation?: Submission["evaluation"]): Promise<Digest> {
        const content =
            build.kind === "build" ? { kind: build.kind, manifest: build.manifest } : build;

        return Digest.json(
            evaluation === undefined
                ? { build: content }
                : {
                      build: content,
                      parameters: evaluation.parameters,
                      definition: evaluation.definition,
                  },
        );
    },

    /** Describe the call keeping a build and its stack evaluation as a revision. */
    async create(
        target: Pick<Installation, "id" | "packageId">,
        build: InstallationBuild,
        evaluation?: Submission["evaluation"],
    ) {
        return installationRevision.call("create", {
            installationId: target.id,
            packageId: target.packageId,
            build,
            ...(evaluation === undefined ? {} : { definition: evaluation.definition }),
            digest: await InstallationRevision.digest(build, evaluation),
        });
    },
};

/** Match the installations a machine launches: enabled, and not being deleted. */
function launchable(): SQL {
    return and(eq(installation.table.status, "enabled"), isNull(installation.table.deletedAt));
}

/** Read the identifier of an installation a call refers to. */
async function identify(
    database: DatabaseConnection,
    installed: { readonly spaceId: string; readonly installation: string },
): Promise<Identifier<"installation"> | undefined> {
    const id = schema.identifier("installation").safeParse(installed.installation);
    const spaceId = schema.identifier("space").safeParse(installed.spaceId);

    // take an identifier as given
    if (id.success) {
        return id.data;
    }
    // find a name in the call's space, served or not
    else if (spaceId.success) {
        const [found] = await database
            .select({ id: installation.table.id })
            .from(installation.table)
            .where(
                and(
                    installation.inScope(spaceId.data),
                    eq(installation.table.name, installed.installation),
                ),
            );

        return found?.id;
    }
    // find none outside a space
    else {
        return undefined;
    }
}

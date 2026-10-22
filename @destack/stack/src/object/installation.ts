import { DeclarationReference } from "@destack/package/declare";
import { ViewPath } from "@destack/view/declare";
import {
    AccessContext,
    Caller,
    Grant,
    type GrantPermission,
    none,
    PermissionReference,
    type PermissionRequest,
    PermissionScope,
    PermissionSet,
    principal,
    self,
} from "@destack/access";
import { InstallationName, type PackageDescription } from "@destack/object";
import {
    and,
    type DatabaseConnection,
    eq,
    isNull,
    Snapshot,
    type Select,
    type SQL,
} from "@destack/db";
import {
    defineObject,
    field,
    grant,
    type MethodContext,
    type ObjectOf,
    Observation,
    role,
} from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import {
    ComputeDefinition,
    DeclarationName,
    DirectoryGrant,
    PackageId,
    type Package,
    PackageRelease,
    Sandbox,
} from "@destack/package";
import { type BuildReader, ViewDescription } from "@destack/package/manifest";
import { Plan } from "@destack/resource";
import {
    canonicalize,
    Digest,
    type Identifier,
    Instant,
    type JsonValue,
    present,
    schema,
} from "@destack/schema";
import { CallError } from "@destack/object";
import { RequestId, Scope } from "@destack/sync";
import { Setting } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import {
    InstallationSelection,
    InstallationDefinition,
    type RepositorySelection,
} from "../declare/installation.ts";
import { build, Build } from "./build.ts";
import type { PackageStore } from "@destack/build/store";
import { StackDefinition } from "../declare/stack.ts";
import { space, Space } from "@destack/account/object";
import { Submission } from "./stack.ts";

/** The name managing an installation's grant, beside the declarations of its stack. */
const GRANT_NAME = "permissions";

/** The account owning the repository of a written selection, null for any other selection. */
export const SelectedAccount = schema.identifier("account").nullable();

/** A view of an installation's revision, as a client opens it. */
export const InstallationView = schema.object({
    /** The installation. */
    installationId: schema.identifier("installation"),
    /** The revision whose build serves the view. */
    revisionId: schema.identifier("installation-revision"),
    /** The digest of the manifest of the build serving the view. */
    manifest: Digest,
    /** The view's name. */
    view: DeclarationName,
    /** The view as the build's output mounts it. */
    description: ViewDescription,
    /** The location within the view. */
    path: ViewPath,
    /** The opening person's home space, where the view's home types open. */
    home: schema.identifier("space"),
    /** What the opening person lends the installation on use, by the scope it applies in. */
    permissions: PermissionSet,
});
/** A view of an installation's revision, as a client opens it. */
export type InstallationView = schema.Infer<typeof InstallationView>;

/** A short-lived access token a space grants one of its installations for a package. */
export const InstallationToken = schema.object({
    /** The signed access token. */
    accessToken: schema.string(),
    /** The HTTP authorization scheme. */
    tokenType: schema.literal("Bearer"),
    /** The exclusive expiry in epoch milliseconds. */
    expiresAt: Instant,
});
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
            /** The revision the space serves, absent before it serves one. */
            servedRevision: field.reference(() => installationRevision).optional(),
            /** The revision applied before the applied one, which a rollback follows again. */
            previousRevision: field.reference(() => installationRevision).optional(),
            /** Whether the installation should serve requests. */
            status: field.enum(["enabled", "suspended"]).default("enabled"),
            /** The space-local address for this installation. */
            name: field.string(InstallationName),
            /** Workload compute settings checked against package and machine policy. */
            compute: field.json(schema.record(DeclarationName, ComputeDefinition)).default({}),
            /** The optional capabilities the space's stack declares the installation allows. */
            capabilities: field.json(schema.array(DeclarationName)).default([]),
            /** The optional capabilities allowed at the installation's request, once approved. */
            allowed: field.json(schema.array(DeclarationName)).default([]),
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
            allow: none(),
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
            output: schema.object({}),
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

            // refuse a revision of a compiled build, whose repository or checkout the installation follows on
            const built = present(
                await call.read(build).findFirst({ where: { id: previous.buildId } }),
                `the build of revision ${previous.id}`,
            );
            if (built.source.kind !== "release") {
                throw new CallError("CONFLICT", {
                    message: `installation ${call.target.name} rolls back only to a release, its previous revision is a build`,
                });
            }

            // follow it and pin its release
            await call.update({
                revisionId: previous.id,
                selection: { kind: "release", version: built.source.version },
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
            /** The space's build evaluated: compiled here or pulled as a release. */
            build: field.reference((): ObjectOf<{ identity: "build" }> => build, {
                delete: "restrict",
            }),
            /** The installable packages its build depends on, at their compiled releases. */
            dependencies: field.json(schema.array(PackageRelease)),
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
            fields: ["installationId", "buildId", "definition", "digest"],
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
    /** List the installations selecting a matching repository reference, in one space or all. */
    async selecting(
        database: DatabaseConnection,
        spaceId: Identifier<"space"> | undefined,
        matches: (selection: RepositorySelection) => boolean,
    ): Promise<
        {
            readonly id: Identifier<"installation">;
            readonly scope: string;
            readonly role: Installation["role"];
            readonly selection: RepositorySelection;
        }[]
    > {
        // read the live installations, of the space if given
        const live = isNull(installation.table.deletedAt);
        const rows = await database
            .select({
                id: installation.table.id,
                scope: installation.table.scope,
                role: installation.table.role,
                selection: installation.table.selection,
            })
            .from(installation.table)
            .where(spaceId === undefined ? live : and(eq(installation.table.scope, spaceId), live));

        return rows.flatMap((each) =>
            each.selection.kind === "repository" && matches(each.selection)
                ? [{ ...each, selection: each.selection }]
                : [],
        );
    },

    /** Read the build a space's installation of a package follows from a store of builds. */
    async reader(
        snapshot: Snapshot,
        spaceId: Identifier<"space">,
        packageId: PackageId,
        installationId: Identifier<"installation"> | undefined,
        packages: Pick<PackageStore, "contents">,
    ): Promise<BuildReader> {
        // read the installation's revision
        const [found] = await snapshot.read(installation.table).findMany({
            where: {
                scope: spaceId,
                packageId,
                ...(installationId === undefined ? {} : { id: installationId }),
            },
        });
        if (found?.revisionId === undefined || found.revisionId === null) {
            throw new CallError("NOT_FOUND", {
                message: `no installation in ${spaceId} follows a build of ${packageId}`,
            });
        }

        return InstallationRevision.reader(snapshot, found.revisionId, packages);
    },

    /** List the optional capabilities an installation allows: declared ones and requested ones. */
    capabilities(row: Pick<Installation, "capabilities" | "allowed">): string[] {
        return [...new Set([...row.capabilities, ...row.allowed])];
    },

    /** List the package's capabilities an installation has: required and allowed optional ones. */
    enabled(
        row: Pick<Installation, "capabilities" | "allowed">,
        description: Pick<PackageDescription, "capabilities"> | undefined,
    ): string[] {
        const allowed = Installation.capabilities(row);

        return Object.entries(description?.capabilities ?? {})
            .filter(([name, capability]) => capability.optional !== true || allowed.includes(name))
            .map(([name]) => name);
    },

    /** List the packages only a package's optional capabilities need installed. */
    optionalPackages(description: PackageDescription | undefined): PackageId[] {
        return Object.values(description?.capabilities ?? {}).flatMap((capability) =>
            capability.optional === true ? (capability.packages ?? []) : [],
        );
    },

    /** List the packages an installation needs installed, its enabled capabilities' included. */
    needed(
        row: Pick<Installation, "name" | "capabilities" | "allowed">,
        revision: Pick<InstallationRevision, "dependencies">,
        description: PackageDescription | undefined,
    ): { readonly package: Package; readonly capability?: string }[] {
        // keep the dependencies optional capabilities alone need out
        const optional = new Set(Installation.optionalPackages(description));
        const required = revision.dependencies
            .filter((dependency) => !optional.has(dependency.package.id))
            .map((dependency) => ({ package: dependency.package }));

        // add the packages each enabled capability needs, which its build depends on
        const enabled = Installation.enabled(row, description).flatMap((capability) =>
            (description?.capabilities[capability]?.packages ?? []).map((packageId) => {
                const dependency = present(
                    revision.dependencies.find((each) => each.package.id === packageId),
                    `${row.name}'s build dependency ${packageId}, which capability ${capability} needs`,
                );

                return { package: dependency.package, capability };
            }),
        );

        return [...required, ...enabled];
    },

    /** Order installations so each comes before the packages its revision depends on. */
    dependentsFirst<
        Row extends Pick<Installation, "packageId"> & {
            readonly dependencies: InstallationRevision["dependencies"] | null;
        },
    >(rows: readonly Row[]): Row[] {
        // take the rows no remaining row depends on, until none remain
        const ordered: Row[] = [];
        let remaining = [...rows];
        while (remaining.length > 0) {
            const needed = new Set(
                remaining.flatMap((row) =>
                    (row.dependencies ?? []).map((dependency) => dependency.package.id),
                ),
            );
            const free = remaining.filter((row) => !needed.has(row.packageId));
            if (free.length === 0) {
                throw new CallError("CONFLICT", {
                    message: `installations of ${remaining.map((row) => row.packageId).join(", ")} depend on each other`,
                });
            }
            ordered.push(...free);
            remaining = remaining.filter((row) => !free.includes(row));
        }

        return ordered;
    },

    /** Join the sandbox abilities an installation's enabled capabilities unlock. */
    sandbox(
        row: Pick<Installation, "capabilities" | "allowed">,
        description: PackageDescription | undefined,
    ): Sandbox {
        return Sandbox.merge(
            ...Installation.enabled(row, description).map(
                (name) => description?.capabilities[name]?.sandbox ?? {},
            ),
        );
    },

    /** Merge what a delegator lends an installation: its package's and capabilities' sets. */
    permissions(
        row: Pick<Installation, "capabilities" | "allowed">,
        description: Pick<PackageDescription, "permissions" | "capabilities"> | undefined,
        delegator: keyof PermissionRequest,
    ): PermissionSet {
        // gather the package's set and each enabled capability's
        const enabled = Installation.enabled(row, description);
        const sets: PermissionSet[] = [
            description?.permissions[delegator] ?? {},
            ...Object.entries(description?.capabilities ?? {})
                .filter(([name]) => enabled.includes(name))
                .map(([, capability]) => capability.permissions?.[delegator] ?? {}),
        ];

        // merge them scope by scope, each permission once
        return Object.fromEntries(
            PermissionScope.options.flatMap((scope) => {
                const merged = sets.flatMap((set) => set[scope] ?? []);
                const once = merged.filter(
                    (permission, index) =>
                        merged.findIndex(
                            (other) =>
                                PermissionReference.key(other) ===
                                PermissionReference.key(permission),
                        ) === index,
                );

                return once.length === 0 ? [] : [[scope, once]];
            }),
        );
    },

    /** Record the revision a space serves each of its installations at, none for the others. */
    async serve(
        store: Pick<ObjectStore, "database" | "mutate" | "principal" | "clock">,
        spaceId: Identifier<"space">,
        revisions: ReadonlyMap<string, string>,
    ): Promise<void> {
        // observe each installation whose served revision changed, as the space's store
        const rows = await Snapshot.live(store.database)
            .read(installation.table)
            .findMany({ where: { scope: spaceId } });
        const calls = rows.flatMap((row) => {
            const served = revisions.get(row.id) ?? null;

            return row.servedRevisionId === served
                ? []
                : [
                      Observation.call(installation, row, {
                          conditions: {},
                          fields: { servedRevisionId: served },
                      }),
                  ];
        });
        if (calls.length > 0) {
            const now = store.clock.now();
            await store.mutate(AccessContext.of(Caller.of(store.principal), now), {
                id: RequestId.create(now),
                calls,
            });
        }
    },

    /** Upsert an installation's role in its space and its space's grant to what it may do. */
    async upsertAccess(
        context: MethodContext,
        target: Installation,
        description: PackageDescription | undefined,
    ): Promise<void> {
        // upsert the role with exactly the space permissions, bound to the installation
        const set = Installation.permissions(target, description, "installation");
        const installed = principal.installation.reference(target.scope, target.id);
        const manager = {
            installationId: target.id,
            packageId: target.packageId,
            name: GRANT_NAME,
        };
        await context.mutate(context.self, [
            role.call("upsertBinding", {
                ...role.in(target.scope),
                name: `installation/${target.id}`,
                description: `What ${target.name} requests in this space`,
                permissions: [...(set.space ?? [])],
                subject: installed,
                manager,
            }),
        ]);

        // upsert the space's grant with exactly the permissions beyond the space
        const [link] = await Scope.read({ read: (table) => context.read(table) }, [target.scope]);
        const found = present(link, `the scope of space ${target.scope}`);
        const permissions: GrantPermission[] = PermissionSet.grant(set, {
            space: target.scope,
            home: [],
            account: found.parent,
        }).filter((permission) => permission.scope !== target.scope);

        // let the installation ask for its package's optional capabilities, each once its space approves
        const isAsking = Object.values(description?.capabilities ?? {}).some(
            (capability) => capability.optional === true,
        );
        const asking = isAsking
            ? [
                  {
                      ...installation.permission("allow"),
                      scope: target.scope,
                      objectId: target.id,
                      mode: "approval" as const,
                  },
              ]
            : [];
        await upsertGrant(context, installed, [...permissions, ...asking]);
    },

    /** Resolve an installation's value of each space setting its build declares, by setting key. */
    async settings(
        database: DatabaseConnection,
        reader: BuildReader,
        target: {
            readonly scope: Identifier<"space">;
            readonly installation: Identifier<"installation">;
        },
    ): Promise<Record<string, JsonValue>> {
        // resolve each space setting the build declares for the installation
        const declared = await Setting.list(reader);
        const placed = declared.filter((setting) => setting.definition.scope === "space");
        const resolved = await Promise.all(
            placed.map(async (setting): Promise<[string, JsonValue]> => [
                DeclarationReference.key(setting.reference),
                schema
                    .json()
                    .parse(await SettingValue.resolve(Snapshot.live(database), setting, target)),
            ]),
        );

        return Object.fromEntries(resolved);
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
    /** Read the build a revision evaluated from a store of builds. */
    async reader(
        snapshot: Pick<Snapshot, "read">,
        revisionId: Identifier<"installation-revision">,
        packages: Pick<PackageStore, "contents">,
    ): Promise<BuildReader> {
        // read the revision, refusing an unknown one
        const revision = await snapshot
            .read(installationRevision.table)
            .findFirst({ where: { id: revisionId } });
        if (revision === undefined) {
            throw new CallError("NOT_FOUND", { message: `no revision ${revisionId}` });
        }

        // read its build from the store
        const built = await Build.require(snapshot, revision.scope, revision.buildId);

        return packages.contents(built.manifest);
    },

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

    /** Digest a build's manifest and its stack evaluation, the same for every build row of it. */
    digest(manifest: string, evaluation?: Submission["evaluation"]): Promise<Digest> {
        return Digest.json(
            evaluation === undefined
                ? { manifest }
                : {
                      manifest,
                      parameters: evaluation.parameters,
                      definition: evaluation.definition,
                  },
        );
    },

    /** Describe the call keeping a space's build and its stack evaluation as a revision. */
    async create(
        target: Pick<Installation, "id" | "packageId">,
        evaluated: { readonly id: Identifier<"build">; readonly manifest: string },
        evaluation?: Submission["evaluation"],
    ) {
        return installationRevision.call("create", {
            installationId: target.id,
            packageId: target.packageId,
            buildId: evaluated.id,
            ...(evaluation === undefined ? {} : { definition: evaluation.definition }),
            digest: await InstallationRevision.digest(evaluated.manifest, evaluation),
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

/** Upsert the space's one grant to its installation giving exactly some permissions. */
async function upsertGrant(
    context: MethodContext,
    installed: ReturnType<typeof principal.installation.reference>,
    permissions: readonly GrantPermission[],
): Promise<void> {
    // leave a grant giving exactly the permissions
    const delegator = principal.space.reference(Scope.universe.id, installed.scope);
    const parties = { delegator, delegate: installed };
    const current = await Grant.find(
        { read: (table) => context.read(table) },
        parties,
        context.now,
    );
    if (current !== undefined && canonicalize(current.permissions) === canonicalize(permissions)) {
        return;
    }

    // replace the outdated grant with the exact one, or revoke it once nothing is requested
    const create = grant.call("create", {
        ...grant.in(installed.scope),
        delegate: installed,
        permissions: [...permissions],
    });
    const revoke = current === undefined ? [] : [grant.call("revoke", grant.target(current))];
    await context.mutate(context.self, permissions.length === 0 ? revoke : [create]);
}

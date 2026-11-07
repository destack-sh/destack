import { StackDefinition } from "../declare/index.ts";
import { ViewPath } from "@destack/view/declare";
import {
    AccessContext,
    Caller,
    none,
    PermissionReference,
    type GrantRequest,
    PermissionScope,
    PermissionSet,
} from "@destack/access";
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
    changeRequest,
    defineObject,
    field,
    Installation,
    installation,
    type MethodContext,
    type ObjectOf,
    PackageDescription,
} from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { DeclarationName, PackageId, Package, PackageRelease, Sandbox } from "@destack/package";
import { type BuildReader, ViewDescription } from "@destack/package/manifest";
import { Plan, ResourceDescription } from "@destack/resource";
import { Digest, type DigestCrypto, Identifier, present, schema, type Uuid } from "@destack/schema";
import { ObjectReference, RequestId, Scope, type Subject } from "@destack/sync";
import { build, Build } from "./build.ts";
import type { BuildStore } from "@destack/build/store";
import { space, Space } from "@destack/account/object";
import { StackEvaluation, Submission } from "./stack.ts";
import { StackError } from "../error/index.ts";
import { Binding } from "./binding.ts";

/** The revision namespace: the DNS namespace's UUIDv8 of installation-revision.destack.app. */
const REVISION_NAMESPACE: Uuid = "472ed1a0-20d1-8e8e-b9d9-c6879123e146";

/** What a submission prepares: the pulled release, and the revision's dependencies and states. */
export const SubmissionPreparation = schema.object({
    /** The pulled release a tag or version selects, null for a space build. */
    release: schema.object({ manifest: Digest, version: schema.string() }).nullable(),
    /** The installable packages the selected build depends on, at their compiled releases. */
    dependencies: schema.array(PackageRelease),
    /** The state each declaration of the selected build requires of its target, as a blob. */
    states: schema.record(schema.string(), Digest),
});
/** What a submission reads before its transaction. */
export type SubmissionPreparation = schema.Infer<typeof SubmissionPreparation>;

/** A resource declaration of an installation's package. */
export const ResourceNeed = ResourceDescription.extend({
    /** The package declaring the resource. */
    package: Package,
});
/** A resource declaration of an installation's package. */
export type ResourceNeed = schema.Infer<typeof ResourceNeed>;

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

/** How an installation rolls out its package: the revisions it targets, applies and serves. */
export const rollout = defineObject(
    {
        name: "rollout",
        plural: "rollouts",
        scope: space,
        key: "installationId",
        fields: {
            /** The installation rolling out, its rollout removed with it. */
            installation: field.reference(() => installation, { delete: "cascade" }).immutable(),
            /** The principal submitting the targeted revision, absent before the first one. */
            submittedBy: field.subject().optional(),
            /** The digest of the space's stack the installation last followed, absent for none. */
            spaceStack: field.string(Digest).optional(),
            /** The revision fully applied, absent before the first application. */
            appliedRevision: field.reference(() => installationRevision).optional(),
            /** The revision the space serves, absent before it serves one. */
            servedRevision: field.reference(() => installationRevision).optional(),
            /** The revision applied before the applied one, which a rollback targets again. */
            previousRevision: field.reference(() => installationRevision).optional(),
            /** The digest of the objects the installation last related as their consumer. */
            consumption: field.string(Digest).optional(),
        },
        permissions: {
            read: none(),
            list: none(),
            create: none(),
            update: none(),
            apply: none(),
            open: none(),
            target: none(),
            deploy: none(),
        },
        // ask approval of a waiting plan through a change request on the rollout
        attachments: [changeRequest.attach({ by: "apply" })],
        constraints: {
            space: {
                kind: "foreignKey",
                on: ["scope"],
                references: { object: () => space, on: ["id"] },
                onDelete: "no action",
            },
            scopeId: { kind: "unique", on: ["scope", "id"] },
            appliedRevisionInstallation: {
                kind: "foreignKey",
                on: ["installationId", "appliedRevisionId"],
                references: { object: () => installationRevision, on: ["installationId", "id"] },
                onDelete: "no action",
            },
            servedRevisionInstallation: {
                kind: "foreignKey",
                on: ["installationId", "servedRevisionId"],
                references: { object: () => installationRevision, on: ["installationId", "id"] },
                onDelete: "no action",
            },
            previousRevisionInstallation: {
                kind: "foreignKey",
                on: ["installationId", "previousRevisionId"],
                references: { object: () => installationRevision, on: ["installationId", "id"] },
                onDelete: "no action",
            },
        },
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("create", { visibility: "installation" }),
        /** Record the revisions the installation applied and serves, and what it consumes. */
        update: method.update("update", {
            visibility: "installation",
            fields: ["appliedRevisionId", "previousRevisionId", "servedRevisionId", "consumption"],
        }),

        /** Read the plan waiting for approval with the digest approving it names. */
        plan: method.query({
            permission: "read",
            output: schema.object({
                /** The steps waiting, across the installation, those it manages and resources. */
                plan: Plan,
                /** The plan's digest, which approving it names. */
                digest: Digest,
                /** The approval its change request asks of the space managers, null until open. */
                approval: schema.identifier("approval").nullable(),
            }),
        }),

        /** Target a submitted build's revision. */
        submit: method.mutation({
            permission: "apply",
            input: Submission,
            prepared: SubmissionPreparation,
        }),

        /** Follow the space's stack: record its digest, dropping an owner's earlier apply. */
        follow: method.mutation({
            permission: "target",
            visibility: "package",
            input: schema.object({
                /** The digest of the space's stack. */
                spaceStack: Digest,
            }),
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

        /** Target the revision applied before the current one again. */
        rollBack: method.mutation({
            permission: "apply",
            output: schema.object({
                /** The revision the installation targets again. */
                revisionId: schema.identifier("installation-revision"),
            }),
        }),

        /** Deploy the revision an application targets, and drain a suspended one. */
        deploy: method.mutation({
            permission: "deploy",
            visibility: "package",
            output: schema.object({}),
        }),

        /** Revise an installation: its build at its bindings, targeted, as its controller does. */
        revise: method.mutation({
            permission: "target",
            visibility: "package",
            input: schema.object({
                /** The build to run. */
                build: schema.identifier("build"),
                /** The installable packages the build depends on, at their compiled releases. */
                dependencies: schema.array(PackageRelease),
                /** The state each declaration of the build requires of its target, as a blob. */
                states: schema.record(schema.string(), Digest),
                /** The stack's evaluation of the build, for a stack. */
                evaluation: StackEvaluation.exactOptional(),
            }),
            output: schema.object({
                /** The targeted revision. */
                revisionId: schema.identifier("installation-revision"),
            }),
        }),
    }),
).implement({
    methods: {
        rollBack: async (call) => {
            // read the revision applied before the applied one, and the installation's name
            const { previousRevisionId } = call.target;
            const { name } = (await Rollout.of(call)).installation;
            const previous =
                previousRevisionId === null
                    ? undefined
                    : await call
                          .read(installationRevision)
                          .findFirst({ where: { id: previousRevisionId } });

            // refuse an installation without one
            if (previous === undefined) {
                throw StackError.of(
                    "PREVIOUS_REVISION_MISSING",
                    `installation ${name} has no revision before its applied one`,
                );
            }

            // refuse a revision of a compiled build, whose repository or checkout the installation follows on
            const built = present(
                await call.read(build).findFirst({ where: { id: previous.buildId } }),
                `the build of revision ${previous.id}`,
            );
            if (built.source.kind !== "release") {
                throw StackError.of(
                    "ROLLBACK_TO_BUILD",
                    `installation ${name} can only roll back to a release, but its previous revision is a build`,
                );
            }

            // select its release on the installation and target it
            const target = installation.target({
                scope: call.target.scope,
                id: call.target.installationId,
            });
            await call.mutate(call.self, [
                installation.call("update", {
                    ...target,
                    selection: { kind: "release", version: built.source.version },
                }),
                installation.call("target", { ...target, revisionId: previous.id }),
            ]);

            return { revisionId: previous.id };
        },
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
            /** The state each declaration requires, such as main's tables, by name. */
            states: field.blobMap(),
            /** The evaluated stack definition, for stacks. */
            definition: field.json(StackDefinition).optional(),
            /** The targets its package's declarations run with, by package and declaration name. */
            bindings: field.json(schema.array(Binding)),
            /** The installation's generation the revision was made at. */
            generation: field.integer(),
            /** The SHA-256 digest of the build, the stack evaluation's inputs and the bindings. */
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
            visibility: "package",
            fields: [
                "installationId",
                "buildId",
                "dependencies",
                "states",
                "definition",
                "bindings",
                "generation",
                "digest",
            ],
            input: schema.object({
                /** The package whose build the revision evaluates. */
                packageId: PackageId,
            }),
        }),
    }),
);

/** How an installation of a space rolls out its package's revisions. */
export type Rollout = Select<typeof rollout.table>;

/** An installation's rollout joined with the installation, never a base one, so with its name. */
export type InstallationRollout = Omit<Rollout, "id"> & {
    /** The revision its installation targets, absent before the first. */
    readonly revisionId: Identifier<"installation-revision"> | null;
    /** The installation the rollout rolls out, with its name in its space. */
    readonly installation: Installation & { readonly name: string };
};

/** The verbs of rollouts and their installations: reads, access, relations and releases. */
export const Rollout = {
    /** Read the rollouts of some installations joined with them, none of a base installation. */
    async installed(
        snapshot: Pick<Snapshot, "read">,
        installed: readonly Installation[],
    ): Promise<InstallationRollout[]> {
        const rows = await snapshot
            .read(rollout.table)
            .findMany({ where: { installationId: { in: installed.map((entry) => entry.id) } } });

        return rows.map((row) =>
            Rollout.join(
                row,
                present(
                    installed.find((entry) => entry.id === row.installationId),
                    `the installation of rollout ${row.installationId}`,
                ),
            ),
        );
    },

    /** Join a rollout with its installation, refusing a base installation. */
    join(row: Rollout, installed: Installation): InstallationRollout {
        // refuse a base installation, which its scope bore without a rollout or a name
        const { name } = installed;
        if (name === null) {
            throw new TypeError(
                `installation ${installed.id} is a base installation without a rollout`,
            );
        }

        return { ...row, revisionId: installed.revisionId, installation: { ...installed, name } };
    },

    /** Read rollouts' installations and join them, refusing a rollout without its installation. */
    async read(
        snapshot: Pick<Snapshot, "read">,
        rows: readonly Rollout[],
    ): Promise<InstallationRollout[]> {
        // read the installations of the rollouts at once
        const installed = await snapshot
            .read(installation.table)
            .findMany({ where: { id: { in: rows.map((row) => row.installationId) } } });
        const byId = new Map(installed.map((row) => [row.id, row]));

        return rows.map((row) =>
            Rollout.join(
                row,
                present(
                    byId.get(row.installationId),
                    `the installation of rollout ${row.installationId}`,
                ),
            ),
        );
    },

    /** Read the rollout a call targets with its installation's fields. */
    async of(
        context: Pick<MethodContext, "read"> & { readonly target: Rollout },
    ): Promise<InstallationRollout> {
        const [joined] = await Rollout.read({ read: (table) => context.read(table) }, [
            context.target,
        ]);

        return present(joined, `the rollout ${context.target.id}`);
    },

    /** Read the manifest of an installation's release selection, none for another selection. */
    pinned(installed: Pick<Installation, "selection">): Digest | undefined {
        const { selection } = installed;

        return selection.kind === "release" ? selection.manifest : undefined;
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
        const allowed = Rollout.capabilities(row);

        return Object.entries(description?.capabilities ?? {})
            .filter(([name, capability]) => capability.optional !== true || allowed.includes(name))
            .map(([name]) => name);
    },

    /** List the packages an installation needs installed, its enabled capabilities' included. */
    needed(
        row: Pick<Installation, "name" | "capabilities" | "allowed">,
        revision: Pick<InstallationRevision, "dependencies">,
        description: PackageDescription | undefined,
    ): { readonly package: Package; readonly capability?: string }[] {
        // leave out the dependencies that only optional capabilities need
        const optional = new Set(Installation.optionalPackages(description));
        const required = revision.dependencies
            .filter((dependency) => !optional.has(dependency.package.id))
            .map((dependency) => ({ package: dependency.package }));

        // add the packages each enabled capability needs, which its build depends on
        const enabled = Rollout.enabled(row, description).flatMap((capability) =>
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
                throw StackError.of(
                    "INSTALLATIONS_CYCLIC",
                    `installations of ${remaining.map((row) => row.packageId).join(", ")} depend on each other`,
                );
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
            ...Rollout.enabled(row, description).map(
                (name) => description?.capabilities[name]?.sandbox ?? {},
            ),
        );
    },

    /** Merge what a delegator lends an installation: its package's and capabilities' sets. */
    permissions(
        row: Pick<Installation, "capabilities" | "allowed">,
        description: Pick<PackageDescription, "permissions" | "capabilities"> | undefined,
        delegator: keyof GrantRequest,
    ): PermissionSet {
        // gather the package's set and each enabled capability's
        const enabled = Rollout.enabled(row, description);
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
        store: Pick<ObjectStore, "database" | "mutate" | "principal" | "clock" | "environment">,
        spaceId: Identifier<"space">,
        revisions: ReadonlyMap<Identifier<"installation">, Identifier<"installation-revision">>,
    ): Promise<void> {
        // observe each installation whose served revision changed, as the space's store
        const rows = await Snapshot.live(store.database)
            .read(rollout.table)
            .findMany({ where: { scope: spaceId } });
        const calls = rows.flatMap((row) => {
            const served = revisions.get(row.installationId) ?? null;

            return row.servedRevisionId === served
                ? []
                : [rollout.call("update", { ...rollout.target(row), servedRevisionId: served })];
        });
        if (calls.length > 0) {
            const now = store.clock.now();
            await store.mutate(AccessContext.of(Caller.of(store.principal), now), {
                id: RequestId.create(store.environment),
                calls,
            });
        }
    },

    /** Read who consents to a rollout's requests: its revision's submitter, else its installer. */
    consenter(target: Omit<Rollout, "id">): Subject {
        return target.submittedById === null
            ? ObjectReference.at("createdBy", target)
            : ObjectReference.at("submittedBy", target);
    },

    /** Read the fields recording a revision applied, with the one applied before as previous. */
    applied(
        target: Pick<Rollout, "appliedRevisionId" | "previousRevisionId">,
        revisionId: Identifier<"installation-revision">,
    ): Pick<Rollout, "appliedRevisionId" | "previousRevisionId"> {
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
    ): Promise<InstallationRollout | undefined> {
        const [joined] = await database
            .select({ rollout: rollout.table, installation: installation.table })
            .from(rollout.table)
            .innerJoin(installation.table, joinedInstallation())
            .innerJoin(space.table, eq(space.table.id, rollout.table.scope))
            .where(and(eq(rollout.table.installationId, id), Space.served(), launchable()));

        return joined === undefined ? undefined : Rollout.join(joined.rollout, joined.installation);
    },

    /** Read where an installation's space moves while a transfer fences it here. */
    async moved(
        database: DatabaseConnection,
        id: Identifier<"installation">,
        spaceId: string,
    ): Promise<{ readonly scope: string; readonly machine: string } | undefined> {
        // read the installation in the call's space
        const [found] = await database
            .select({ scope: installation.table.scope })
            .from(installation.table)
            .where(and(eq(installation.table.id, id), eq(installation.table.scope, spaceId)));
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
    ): Promise<InstallationRollout> {
        // take the installation served in the call's space, by its identifier or its name
        const id = await identify(database, installed);
        const served = id === undefined ? undefined : await Rollout.findServed(database, id);
        if (served?.scope === installed.spaceId) {
            return served;
        }

        // answer a moving space's installation with where it moves
        const moved =
            served === undefined && id !== undefined
                ? await Rollout.moved(database, id, installed.spaceId)
                : undefined;
        // send a moving space's caller to where it moves
        if (moved !== undefined) {
            throw StackError.of("SCOPE_MOVED", `${moved.scope} moved to ${moved.machine}`, {
                data: moved,
            });
        }
        // let a caller wait for an installation not served yet, such as one still deploying
        else if (id !== undefined && served === undefined) {
            throw StackError.of(
                "INSTALLATION_NOT_SERVED",
                `installation ${installed.installation} in this space is not served yet`,
            );
        }
        // refuse an installation this machine does not have
        else {
            throw StackError.of(
                "INSTALLATION_NOT_HERE",
                `this machine does not serve installation ${installed.installation} in this space`,
            );
        }
    },

    /** Find a space's enabled installation not being deleted by its name. */
    async find(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
        name: string,
    ): Promise<InstallationRollout | undefined> {
        const [found] = await database
            .select({ rollout: rollout.table, installation: installation.table })
            .from(rollout.table)
            .innerJoin(installation.table, joinedInstallation())
            .where(and(rollout.inScope(spaceId), eq(installation.table.name, name), launchable()));

        return found === undefined ? undefined : Rollout.join(found.rollout, found.installation);
    },

    /** List a space's enabled, undeleted installations, oldest first, with their revisions. */
    async list(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
    ): Promise<
        {
            rollout: InstallationRollout;
            revision: InstallationRevision;
        }[]
    > {
        const rows = await database
            .select({
                rollout: rollout.table,
                installation: installation.table,
                revision: installationRevision.table,
            })
            .from(rollout.table)
            .innerJoin(installation.table, joinedInstallation())
            .innerJoin(
                installationRevision.table,
                eq(installationRevision.table.id, installation.table.revisionId),
            )
            .where(and(rollout.inScope(spaceId), launchable()))
            .orderBy(installation.table.createdAt, installation.table.id);

        return rows.map((row) => ({
            rollout: Rollout.join(row.rollout, row.installation),
            revision: row.revision,
        }));
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
        packages: Pick<BuildStore, "contents">,
    ): Promise<BuildReader> {
        // read the revision, refusing an unknown one
        const revision = await snapshot
            .read(installationRevision.table)
            .findFirst({ where: { id: revisionId } });
        if (revision === undefined) {
            throw StackError.of("REVISION_NOT_FOUND", `revision ${revisionId} not found`);
        }

        // read its build from the store
        const built = await Build.require(snapshot, revision.scope, revision.buildId);

        return packages.contents(built.manifest);
    },

    /** Read the revision an installation follows, absent before its first. */
    async followed(
        database: DatabaseConnection,
        target: Pick<InstallationRollout, "revisionId">,
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

    /** Digest a build's manifest and its stack evaluation's inputs, which decide its definition. */
    digest(
        manifest: Digest,
        inputs: Omit<StackEvaluation, "definition"> | undefined,
        bindings: readonly Binding[],
        crypto: DigestCrypto,
    ): Promise<Digest> {
        return Digest.json(
            inputs === undefined
                ? { manifest, bindings }
                : {
                      manifest,
                      export: inputs.export,
                      parameters: inputs.parameters,
                      settings: inputs.settings,
                      bindings,
                  },
            crypto,
        );
    },

    /** Identify an installation's revision of a digest, the same everywhere, as git its objects. */
    id(
        installationId: Identifier<"installation">,
        digest: Digest,
        crypto: DigestCrypto,
    ): Promise<Identifier<"installation-revision">> {
        const name = `${installationId}/${digest}`;

        return Identifier.hash("installation-revision", REVISION_NAMESPACE, name, crypto);
    },
};

/** Match the installations a machine launches: enabled, and their rollouts not being deleted. */
function launchable(): SQL {
    return and(eq(installation.table.status, "enabled"), isNull(installation.table.deletedAt));
}

/** Join a rollout's installation, which shares its identifier and scope. */
function joinedInstallation(): SQL {
    return and(
        eq(installation.table.id, rollout.table.installationId),
        eq(installation.table.scope, rollout.table.scope),
    );
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

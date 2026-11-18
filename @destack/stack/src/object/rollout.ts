import { RequestedPermission } from "@destack/account/authentication";
import { StackDefinition } from "../declare/index.ts";
import { ViewPath } from "@destack/view/declare";
import {
    AccessContext,
    Caller,
    none,
    PermissionReference,
    type GrantDeclaration,
    type GrantContext,
    PermissionSet,
} from "@destack/access";
import {
    and,
    type DatabaseConnection,
    eq,
    isNotNull,
    isNull,
    Snapshot,
    type Select,
    type SQL,
} from "@destack/db";
import {
    changeRequest,
    type Condition,
    defineObject,
    field,
    Installation,
    installation,
    type MethodContext,
    ObjectDescription,
    Observation,
    type ObservationReport,
    type ObjectOf,
    type ObjectType,
    PackageDescription,
} from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import {
    DeclarationName,
    PackageId,
    PackageVersion,
    PackageBuild,
    Sandbox,
} from "@destack/package";
import { type BuildReader, ViewDescription } from "@destack/package/manifest";
import { Plan, ResourceDescription } from "@destack/resource";
import { Digest, type DigestCrypto, Identifier, present, schema, type Uuid } from "@destack/schema";
import {
    ObjectReference,
    ObjectTypeReference,
    RequestId,
    Scope,
    type Subject,
} from "@destack/sync";
import { build, Build } from "./build.ts";
import { space, Space } from "@destack/account/object";
import { StackEvaluation } from "./stack.ts";
import { StackError } from "../error/index.ts";
import { Failure } from "@destack/error";
import { Binding } from "./binding.ts";
import { argument, plural, t } from "@destack/locale";

/** The name of the installation applying a space's stack from its creation. */
export const STACK_INSTALLATION = "stack";

/** The milliseconds a caller waits for an installation not served yet: a deploy takes seconds. */
const NOT_SERVED_RETRY_MILLISECONDS = 1000;

/** The revision namespace: the DNS namespace's UUIDv8 of installation-revision.destack.app. */
const REVISION_NAMESPACE: Uuid = "472ed1a0-20d1-8e8e-b9d9-c6879123e146";

/** A resource declaration of an installation's package. */
export const ResourceNeed = ResourceDescription.extend({
    /** The package declaring the resource. */
    package: PackageVersion,
    /** The resource's kind, such as database. */
    kind: DeclarationName,
    /** The package defining the resource's kind. */
    kindPackageId: PackageId,
});
/** A resource declaration of an installation's package. */
export type ResourceNeed = schema.Infer<typeof ResourceNeed>;

/** A view of an installation's revision, as a client opens it. */
export const InstallationView = schema.object({
    /** The installation. */
    installationId: schema.identifier("installation"),
    /** The revision whose build serves the view. */
    revisionId: schema.identifier("installation-revision"),
    /** The digest of the lock of the build serving the view. */
    lock: Digest,
    /** The view's name. */
    view: DeclarationName,
    /** The installed package's name, such as @acme/notes. */
    package: schema.string().min(1),
    /** The view as the build's output mounts it. */
    description: ViewDescription,
    /** The location within the view. */
    path: ViewPath,
    /** The opening person's home space, where the view's home types open. */
    home: schema.identifier("space"),
    /** What the opening person lends the installation on use, each as they read it. */
    permissions: schema.array(RequestedPermission),
});
/** A view of an installation's revision, as a client opens it. */
export type InstallationView = schema.Infer<typeof InstallationView>;

/** How an installation rolls out its package: the revisions it targets, applies and serves. */
export const rollout = defineObject(
    {
        name: "rollout",
        plural: "rollouts",
        title: t`${plural(argument("count"), { one: "Rollout", other: "Rollouts" })}`,
        scope: space,
        key: "installationId",
        fields: {
            /** The installation rolling out, its rollout removed with it. */
            installation: field.reference(() => installation, { delete: "cascade" }).immutable(),
            /** The principal submitting the targeted revision, absent before the first one. */
            submittedBy: field.subject().optional(),
            /** The digest of the space's stack the installation last followed, absent for none. */
            spaceStack: field.string(Digest).optional(),
            /** The revision apply decided it runs, absent before the first. */
            targetRevision: field.reference(() => installationRevision).optional(),
            /** The revision fully applied, absent before the first application. */
            appliedRevision: field.reference(() => installationRevision).optional(),
            /** The revision the space serves, absent before it serves one. */
            servedRevision: field.reference(() => installationRevision).optional(),
            /** The digest of the objects the installation last related as their consumer. */
            consumption: field.string(Digest).optional(),
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
            targetRevisionInstallation: {
                kind: "foreignKey",
                on: ["installationId", "targetRevisionId"],
                references: { object: () => installationRevision, on: ["installationId", "id"] },
                onDelete: "no action",
            },
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
        },
        access: {
            permissions: {
                read: none(),
                list: none(),
                create: none(),
                update: none(),
                apply: none(),
                open: none(),
            },
        },
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("create", { visibility: "installation" }),
        /** Record the revisions the installation targets, applied and serves. */
        update: method.update("update", {
            visibility: "installation",
            fields: ["targetRevisionId", "appliedRevisionId", "servedRevisionId", "consumption"],
        }),

        /** Read the plan waiting for approval with the digest approving it names. */
        plan: method.query({
            title: t`Plan`,
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

        /** Target the revision of what the installation follows and apply what its plan allows. */
        deploy: method.mutation({
            title: t`Deploy`,
            permission: "apply",
            input: schema.object({
                /** The space's build to target, absent to take the build of what it follows. */
                build: schema.identifier("build").exactOptional(),
            }),
        }),

        /** Follow the space's stack: record its digest, dropping an owner's earlier apply. */
        follow: method.mutation({
            title: t`Follow`,
            permission: "update",
            visibility: "package",
            input: schema.object({
                /** The digest of the space's stack. */
                spaceStack: Digest,
            }),
        }),

        /** Approve the plan waiting for approval by its digest, then apply it. */
        approve: method.mutation({
            title: t`Approve`,
            permission: "apply",
            input: schema.object({
                /** The digest of the reviewed plan. */
                plan: Digest,
            }),
        }),

        /** Read the build and view a client opens at a location, in the opening person's home. */
        open: method.query({
            title: t`Open`,
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

        /** Describe a permission the installation may ask a person for, as they read it. */
        request: method.query({
            title: t`Request`,
            permission: "open",
            input: schema.object({
                /** The permission a call of the installation needs. */
                permission: PermissionReference,
            }),
            output: schema.array(RequestedPermission),
        }),
    }),
);

/** An immutable evaluation of an installation's selected build. */
export const installationRevision = defineObject(
    {
        name: "installation-revision",
        plural: "installationRevisions",
        title: t`${plural(argument("count"), { one: "Installation revision", other: "Installation revisions" })}`,
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
            /** The installable packages its build depends on, at the builds its lock has. */
            dependencies: field.json(schema.array(PackageBuild)),
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
        constraints: {
            installationId: { kind: "unique", on: ["installationId", "id"] },
            digest: { kind: "unique", on: ["installationId", "digest"] },
            digestForm: {
                kind: "check",
                condition: { digest: { length: 64, characters: "0-9a-f" } },
            },
        },
        access: {
            permissions: ["read", "record"],
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
    /** The installation the rollout rolls out. */
    readonly installation: Installation;
};

/** Why a space does not serve an installation, as its Serving condition's reason. */
export const ServingReason = schema.enum([
    "Refused",
    "WaitingForTypes",
    "LoadFailed",
    "StartFailed",
]);
/** Why a space does not serve an installation. */
export type ServingReason = schema.Infer<typeof ServingReason>;

/** What a space serves of an installation: a revision, or why it serves none. */
export type Serving =
    | { readonly revisionId: Identifier<"installation-revision"> }
    | { readonly reason: ServingReason; readonly message: string };

/** Why an installation is not ready yet, as a Kubernetes condition's reason. */
export const WaitingReason = schema.enum([
    "WaitingForResources",
    "WaitingForSettings",
    "WaitingForSpace",
    "WaitingForStack",
    "WaitingForBuild",
]);
/** Why an installation is not ready yet. */
export type WaitingReason = schema.Infer<typeof WaitingReason>;

/** The verbs of rollouts and their installations: reads, access, relations and releases. */
export const Rollout = {
    /** Describe an installation's condition waiting for a reason. */
    waiting(reason: WaitingReason, message: string): Condition {
        return { status: "false", reason, message };
    },

    /** Describe an installation's condition waiting for a lock's builds this machine lacks. */
    waitingForBuild(lock: Digest): Condition {
        const message = `waiting for build ${lock}, which this machine does not have yet`;

        return Rollout.waiting("WaitingForBuild", message);
    },

    /** Report whether one following the host's release runs a lock this machine does not. */
    isSuperseded(
        installed: Pick<Installation, "followsId">,
        lock: Digest,
        trusted: ReadonlySet<Digest>,
    ): boolean {
        return installed.followsId === null && !trusted.has(lock);
    },

    /** Read the rollouts of some installations joined with them. */
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

    /** Join a rollout with its installation. */
    join(row: Rollout, installed: Installation): InstallationRollout {
        return { ...row, installation: installed };
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
    ): { readonly package: PackageVersion; readonly capability?: string }[] {
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
                    "INSTALLATIONS_INVALID",
                    `installations of '${remaining.map((row) => row.packageId).join(", ")}' are invalid: they depend on each other`,
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

    /** Grant a set in the context's scopes and describe each permission as a person reads it. */
    describe(
        set: PermissionSet,
        context: GrantContext,
        objects: readonly ObjectType[],
    ): RequestedPermission[] {
        // key the known types by their policy: their own, or the principal kind they represent
        const described = new Map(
            objects.map((type) => {
                const description = ObjectDescription.of(type);

                return [ObjectDescription.key(description), description] as const;
            }),
        );

        // describe each by its type's policy, refusing one of a policy no known type keys
        return PermissionSet.grant(set, context).map(({ within, permission }) => {
            // find the type keyed by the permission's policy
            const key = ObjectTypeReference.key({
                packageId: permission.packageId,
                type: permission.type,
            });
            const declaring = described.get(key);
            if (declaring === undefined) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `type '${key}' is invalid: the space serves no such type`,
                );
            }

            return {
                ...permission,
                within,
                description: ObjectDescription.describe(declaring, permission.name),
            };
        });
    },

    /** Merge what a delegator lends an installation: its package's and capabilities' sets. */
    permissions(
        row: Pick<Installation, "capabilities" | "allowed">,
        description: Pick<PackageDescription, "permissions" | "capabilities"> | undefined,
        delegator: keyof GrantDeclaration,
    ): PermissionSet {
        // ask nothing without a package, else what it asks with the installation's capabilities
        return description === undefined
            ? {}
            : PackageDescription.asks(description, Rollout.enabled(row, description), delegator);
    },

    /** Record what a space serves of each installation: a revision, or why it serves none. */
    async serve(
        store: Pick<ObjectStore, "database" | "mutate" | "principal" | "clock" | "environment">,
        spaceId: Identifier<"space">,
        serving: ReadonlyMap<Identifier<"installation">, Serving>,
    ): Promise<void> {
        // read the space's rollouts with their installations
        const snapshot = Snapshot.live(store.database);
        const rows = await Rollout.read(
            snapshot,
            await snapshot.read(rollout.table).findMany({ where: { scope: spaceId } }),
        );

        // record each changed served revision, and each changed Serving of an installation the space tries to serve
        const calls = rows.flatMap((row) => {
            // update the served revision once it moves
            const outcome = serving.get(row.installationId);
            const served =
                outcome !== undefined && "revisionId" in outcome ? outcome.revisionId : null;
            const target = rollout.target({ scope: row.scope, id: row.installationId });
            const revised =
                row.servedRevisionId === served
                    ? []
                    : [rollout.call("update", { ...target, servedRevisionId: served })];

            // observe the installation's Serving once it changes
            const observation = outcome === undefined ? undefined : Rollout.serving(outcome);
            const observed =
                observation === undefined || Observation.isRecorded(row.installation, observation)
                    ? []
                    : [Observation.call(installation, row.installation, observation)];

            return [...revised, ...observed];
        });
        const context = AccessContext.of(Caller.of(store.principal), store.clock.now());
        if (calls.length > 0) {
            await store.mutate(context, { id: RequestId.create(store.environment), calls });
        }

        // decide whether any served revision moved, which roles, grants, deployments and declared records follow
        const isMoved = rows.some((row) => {
            const outcome = serving.get(row.installationId);
            const served =
                outcome !== undefined && "revisionId" in outcome ? outcome.revisionId : null;

            return served !== row.servedRevisionId;
        });

        // list the live ones the space serves that moved or still wait
        const moved = rows.filter((row) => {
            // read the revision the space serves of it
            const outcome = serving.get(row.installationId);
            const served =
                outcome !== undefined && "revisionId" in outcome ? outcome.revisionId : null;
            const isLive = row.installation.deletedAt === null;
            const isUnsettled = row.appliedRevisionId !== row.targetRevisionId;

            return (
                isLive &&
                isMoved &&
                served !== null &&
                (isUnsettled || served !== row.servedRevisionId)
            );
        });

        // deploy each one in its own mutation, its refusal recorded by its rollout's pass
        for (const row of moved) {
            const deploying = rollout.call(
                "deploy",
                rollout.target({ scope: row.scope, id: row.installationId }),
            );
            await store
                .mutate(context, { id: RequestId.create(store.environment), calls: [deploying] })
                .catch((error: unknown) => {
                    if (!Failure.isDeclared(error)) {
                        throw error;
                    }
                });
        }
    },

    /** Read an installation's current Serving failure, absent while served or waiting for types. */
    servingFailure(installed: Installation): Condition | undefined {
        const serving = installed.conditions["Serving"];
        const isFailed =
            serving !== undefined &&
            Observation.isCurrent(installed, "Serving") &&
            serving.status === "false" &&
            serving.reason !== "WaitingForTypes";

        return isFailed ? serving : undefined;
    },

    /** Describe a serving outcome as an installation's Serving condition. */
    serving(outcome: Serving): ObservationReport {
        const condition: Condition =
            "revisionId" in outcome
                ? {
                      status: "true",
                      reason: "Served",
                      message: `serving revision ${outcome.revisionId}`,
                  }
                : { status: "false", reason: outcome.reason, message: outcome.message };

        return { conditions: { Serving: condition } };
    },

    /** Read who consents to a rollout's requests: its revision's submitter, else its installer. */
    consenter(target: Omit<Rollout, "id">): Subject {
        return target.submittedById === null
            ? ObjectReference.at("createdBy", target)
            : ObjectReference.at("submittedBy", target);
    },

    /** Read an installation a machine serves now, at a revision its space records as served. */
    async findServed(
        database: DatabaseConnection,
        id: Identifier<"installation">,
    ): Promise<InstallationRollout | undefined> {
        const [joined] = await database
            .select({ rollout: rollout.table, installation: installation.table })
            .from(rollout.table)
            .innerJoin(installation.table, joinedInstallation())
            .innerJoin(space.table, eq(space.table.id, rollout.table.scope))
            .where(
                and(
                    eq(rollout.table.installationId, id),
                    isNotNull(rollout.table.servedRevisionId),
                    Space.served(),
                    launchable(),
                ),
            );

        return joined === undefined ? undefined : Rollout.join(joined.rollout, joined.installation);
    },

    /** Read where an installation's space moves while a transfer fences it here. */
    async moved(
        database: DatabaseConnection,
        id: Identifier<"installation">,
        spaceId: Identifier<"space">,
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
        const spaceId = schema.identifier("space").parse(installed.spaceId);
        const id = await identify(database, spaceId, installed.installation);
        const served = id === undefined ? undefined : await Rollout.findServed(database, id);
        if (served?.scope === spaceId) {
            return served;
        }

        // answer a moving space's installation with where it moves
        const moved =
            served === undefined && id !== undefined
                ? await Rollout.moved(database, id, spaceId)
                : undefined;
        // send a moving space's caller to where it moves
        if (moved !== undefined) {
            throw StackError.of(
                "SCOPE_MOVED",
                `space '${moved.scope}' moved to '${moved.machine}'`,
                {
                    data: moved,
                },
            );
        }
        // let a caller wait for an installation not served yet, such as one still deploying or one the space's stack may still create
        else if (
            (id !== undefined && served === undefined) ||
            (await isStackPending(database, spaceId))
        ) {
            throw StackError.of(
                "INSTALLATION_UNAVAILABLE",
                `installation '${installed.installation}' is unavailable: this space does not serve it yet`,
                { data: { retryAfter: NOT_SERVED_RETRY_MILLISECONDS } },
            );
        }
        // refuse an installation this machine does not have
        else {
            throw StackError.of(
                "INSTALLATION_NOT_FOUND",
                `installation '${installed.installation}' served by this machine in this space not found`,
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
                eq(installationRevision.table.id, rollout.table.targetRevisionId),
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
    /** Read the build row a revision evaluated. */
    async build(
        snapshot: Pick<Snapshot, "read">,
        revisionId: Identifier<"installation-revision">,
    ): Promise<Select<typeof build.table> & { readonly lock: Digest }> {
        // read the revision, refusing an unknown one
        const revision = await snapshot
            .read(installationRevision.table)
            .findFirst({ where: { id: revisionId } });
        if (revision === undefined) {
            throw StackError.of("REVISION_NOT_FOUND", `revision '${revisionId}' not found`);
        }

        return await Build.require(snapshot, revision.scope, revision.buildId);
    },

    /** Read the build a revision of a package evaluated from the space's builds. */
    async reader(
        snapshot: Pick<Snapshot, "read">,
        revisionId: Identifier<"installation-revision">,
        packageId: PackageId,
        builds: { readonly root: (packageId: PackageId, lock: Digest) => Promise<BuildReader> },
    ): Promise<BuildReader> {
        const built = await InstallationRevision.build(snapshot, revisionId);

        return builds.root(packageId, built.lock);
    },

    /** Read the revision an installation follows, absent before its first. */
    async followed(
        database: DatabaseConnection,
        target: Pick<InstallationRollout, "targetRevisionId">,
    ): Promise<InstallationRevision | undefined> {
        if (target.targetRevisionId === null) {
            return undefined;
        }
        const [revision] = await database
            .select()
            .from(installationRevision.table)
            .where(eq(installationRevision.table.id, target.targetRevisionId));

        return revision;
    },

    /** Digest a build's lock and its stack evaluation's inputs, which decide its definition. */
    digest(
        lock: Digest,
        inputs: Omit<StackEvaluation, "definition"> | undefined,
        bindings: readonly Binding[],
        crypto: DigestCrypto,
    ): Promise<Digest> {
        return Digest.json(
            inputs === undefined
                ? { lock, bindings }
                : {
                      lock,
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

/** Determine whether a space's stack may still create installations, before or during a pass. */
async function isStackPending(
    database: DatabaseConnection,
    spaceId: Identifier<"space">,
): Promise<boolean> {
    const stack = await Snapshot.live(database)
        .read(installation.table)
        .findFirst({ where: { scope: spaceId, name: STACK_INSTALLATION } });

    return stack === undefined || Observation.progress(stack) === "waiting";
}

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
    spaceId: Identifier<"space">,
    reference: string,
): Promise<Identifier<"installation"> | undefined> {
    // take an identifier as given
    const id = schema.identifier("installation").safeParse(reference);
    if (id.success) {
        return id.data;
    }

    // find a name in the call's space, served or not
    const [found] = await database
        .select({ id: installation.table.id })
        .from(installation.table)
        .where(and(installation.inScope(spaceId), eq(installation.table.name, reference)));

    return found?.id;
}

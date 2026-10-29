import { DeclarationReference } from "@destack/package/declare";
import { installableReleases, MAIN_DATABASE, StackDefinition } from "../declare/index.ts";
import { ViewPath } from "@destack/view/declare";
import {
    AccessContext,
    Caller,
    Grant,
    type GrantPermission,
    Manager,
    none,
    PermissionReference,
    type PermissionRequest,
    PermissionScope,
    PermissionSet,
    principal,
} from "@destack/access";
import {
    and,
    type BlobStore,
    type DatabaseConnection,
    DatabaseState,
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
    Installation,
    installation,
    InstallationBindingSpec,
    type InstallationBindingState,
    InstallationKind,
    type InstallationSelection,
    type MethodContext,
    ObjectDescription,
    type ObjectOf,
    type ObjectReconciliation,
    type ObjectType,
    ObjectTypePool,
    PackageDescription,
    role,
} from "@destack/object";
import { ObjectStore } from "@destack/object/server";
import { DeclarationName, PackageId, Package, PackageRelease, Sandbox } from "@destack/package";
import { type BuildReader, PackageManifest, ViewDescription } from "@destack/package/manifest";
import { Plan, RESOURCE_DECLARATION, ResourceDescription } from "@destack/resource";
import {
    canonicalize,
    Digest,
    type DigestCrypto,
    Identifier,
    type JsonValue,
    present,
    schema,
    type Uuid,
    Version,
} from "@destack/schema";
import {
    type Call,
    ObjectReference,
    ObjectTypeReference,
    RequestId,
    Scope,
    type Subject,
} from "@destack/sync";
import { Setting } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import { build, Build, type BuildSource } from "./build.ts";
import accountPackage from "@destack/account/package";
import type { BuildStore, Registry } from "../server/installation/build.ts";
import type { Forge } from "../server/installation/follow.ts";
import type { PackageStore } from "@destack/build/store";
import { residency, space, Space } from "@destack/account/object";
import { Submission } from "./stack.ts";
import { StackError } from "../error/index.ts";

/** The name under which an installation manages its permissions binding and space grant. */
const GRANT_NAME = "permissions";

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
        identity: "installation",
        scope: space,
        fields: {
            /** The revision to apply, absent before the first submission. */
            revision: field.reference(() => installationRevision).optional(),
            /** The principal submitting the targeted revision, absent before the first one. */
            submittedBy: field.subject().optional(),
            /** The digest of the space's stack the installation last submitted, absent for none. */
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
        constraints: {
            space: {
                kind: "foreignKey",
                on: ["scope"],
                references: { object: () => space, on: ["id"] },
                onDelete: "no action",
            },
            installation: {
                kind: "foreignKey",
                on: ["scope", "id"],
                references: { object: () => installation, on: ["scope", "id"] },
                onDelete: "cascade",
            },
            scopeId: { kind: "unique", on: ["scope", "id"] },
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
            servedRevisionInstallation: {
                kind: "foreignKey",
                on: ["id", "servedRevisionId"],
                references: { object: () => installationRevision, on: ["installationId", "id"] },
                onDelete: "no action",
            },
            previousRevisionInstallation: {
                kind: "foreignKey",
                on: ["id", "previousRevisionId"],
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

        /** Read the plan waiting for approval. */
        plan: method.query({ permission: "read", output: Plan }),

        /** Target a submitted build's revision. */
        submit: method.mutation({
            permission: "apply",
            input: Submission,
            prepared: SubmissionPreparation,
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

        /** Target a revision for an installation, as its controller does. */
        target: method.mutation({
            permission: "target",
            visibility: "package",
            input: schema.object({
                /** The revision to target. */
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

            // target it and select its release on the installation
            await call.update({ revisionId: previous.id });
            await call.mutate(call.self, [
                installation.call("update", {
                    ...installation.in(call.target.scope),
                    id: call.target.id,
                    selection: { kind: "release", version: built.source.version },
                }),
            ]);

            return { revisionId: previous.id };
        },
        target: (call) => call.update(call.input),
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
            visibility: "package",
            fields: ["installationId", "buildId", "dependencies", "states", "definition", "digest"],
            input: schema.object({
                /** The package whose build the revision evaluates. */
                packageId: PackageId,
            }),
        }),
    }),
);

/** How an installation of a space rolls out its package's revisions. */
export type Rollout = Select<typeof rollout.table>;

/** One change to an installation's access: the calls one caller makes. */
export interface RolloutAccess {
    /** The caller making the calls: the consenter, or the space lending its authority. */
    readonly caller: AccessContext;
    /** The calls upserting the role or the grant. */
    readonly calls: readonly Call[];
}

/** Who builds an installation's access, and when. */
export interface RolloutActor {
    /** The code making the changes. */
    readonly subject: Subject;
    /** The time of the changes. */
    readonly now: number;
}

/** An installation's rollout joined with the installation, never a base one, so with its name. */
export type InstallationRollout = Rollout & {
    /** The installation the rollout rolls out, with its name in its space. */
    readonly installation: Installation & { readonly name: string };
};

/** What targeting and describing revisions read: builds, the registry, blobs and the forge. */
export interface RevisionOptions {
    /** The space's builds, read through to the machine's cache. */
    readonly builds: BuildStore;
    /** The registry of published releases. */
    readonly registry: Registry;
    /** The space's blobs, storing the states its revisions require. */
    readonly blobs: BlobStore;
    /** The forge whose references and tags installations select, null without one. */
    readonly forge: Forge | null;
}

/** The build a selection resolves to: the space's build, its manifest and source. */
interface SelectedBuild {
    /** The space's build. */
    readonly build: Identifier<"build">;
    /** The digest of the build's manifest. */
    readonly manifest: Digest;
    /** Where the build comes from. */
    readonly source: BuildSource;
}

/** The verbs of rollouts and their installations: reads, access, relations and releases. */
export const Rollout = {
    /** Read the rollouts of some installations joined with them, none of a base installation. */
    async installed(
        snapshot: Pick<Snapshot, "read">,
        installed: readonly Installation[],
    ): Promise<InstallationRollout[]> {
        const rows = await snapshot
            .read(rollout.table)
            .findMany({ where: { id: { in: installed.map((entry) => entry.id) } } });

        return rows.map((row) =>
            Rollout.join(
                row,
                present(
                    installed.find((entry) => entry.id === row.id),
                    `the installation of rollout ${row.id}`,
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

        return { ...row, installation: { ...installed, name } };
    },

    /** Read rollouts' installations and join them, refusing a rollout without its installation. */
    async read(
        snapshot: Pick<Snapshot, "read">,
        rows: readonly Rollout[],
    ): Promise<InstallationRollout[]> {
        // read the installations of the rollouts at once
        const installed = await snapshot
            .read(installation.table)
            .findMany({ where: { id: { in: rows.map((row) => row.id) } } });
        const byId = new Map(installed.map((row) => [row.id, row]));

        return rows.map((row) =>
            Rollout.join(row, present(byId.get(row.id), `the installation of rollout ${row.id}`)),
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

    /** List the live installations with a matching selection, in one space or all. */
    async selecting(
        database: DatabaseConnection,
        spaceId: Identifier<"space"> | undefined,
        matches: (selection: InstallationSelection) => boolean,
    ): Promise<
        {
            readonly id: Identifier<"installation">;
            readonly scope: string;
            readonly packageId: PackageId;
            readonly managerInstallationId: Identifier<"installation"> | null;
            readonly selection: InstallationSelection;
        }[]
    > {
        // read the live rollouts with their installations' selections, of the space if given
        const live = isNull(installation.table.deletedAt);
        const rows = await database
            .select({
                id: rollout.table.id,
                scope: rollout.table.scope,
                packageId: installation.table.packageId,
                managerInstallationId: installation.table.managerInstallationId,
                selection: installation.table.selection,
            })
            .from(rollout.table)
            .innerJoin(
                installation.table,
                and(
                    eq(installation.table.id, rollout.table.id),
                    eq(installation.table.scope, rollout.table.scope),
                ),
            )
            .where(spaceId === undefined ? live : and(eq(rollout.table.scope, spaceId), live));

        return rows.filter((each) => matches(each.selection));
    },

    /** Read the build a space's installation of a package follows from a store of builds. */
    async reader(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        packageId: PackageId,
        installationId: Identifier<"installation"> | undefined,
        packages: Pick<PackageStore, "contents">,
    ): Promise<BuildReader> {
        // read the installation's rollout, refusing several of the package unless one is named
        const installed = await snapshot.read(installation.table).findMany({
            where: {
                scope: spaceId,
                packageId,
                ...(installationId === undefined ? {} : { id: installationId }),
            },
        });
        const rows = await snapshot
            .read(rollout.table)
            .findMany({ where: { id: { in: installed.map((row) => row.id) } } });
        const [found] = rows;
        if (rows.length > 1) {
            throw StackError.of(
                "INSTALLATION_AMBIGUOUS",
                `several installations in ${spaceId} follow builds of ${packageId}: pick one`,
            );
        } else if (found?.revisionId === undefined || found.revisionId === null) {
            throw StackError.of(
                "INSTALLATION_NOT_FOUND",
                `no installation in ${spaceId} follows a build of ${packageId}`,
            );
        }

        return InstallationRevision.reader(snapshot, found.revisionId, packages);
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
        delegator: keyof PermissionRequest,
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
            const served = revisions.get(row.id) ?? null;

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
    consenter(target: Rollout): Subject {
        return target.submittedById === null
            ? ObjectReference.at("createdBy", target)
            : ObjectReference.at("submittedBy", target);
    },

    /** Upsert an installation's roles in its space and its space's grant, each as its caller. */
    async upsertAccess(
        context: MethodContext,
        target: InstallationRollout,
        description: PackageDescription | undefined,
    ): Promise<void> {
        // make each change as its caller, with the code running the call as the actor
        const reader: Pick<Snapshot, "read"> = { read: (table) => context.read(table) };
        const changes = await Rollout.access(reader, target, description, {
            subject: context.self.subject,
            now: context.now,
        });
        for (const change of changes) {
            await context.mutate(change.caller, change.calls);
        }
    },

    /** Build the calls upserting an installation's roles and space grant, with their callers. */
    async access(
        reader: Pick<Snapshot, "read">,
        target: InstallationRollout,
        description: PackageDescription | undefined,
        actor: RolloutActor,
    ): Promise<RolloutAccess[]> {
        // act for the consenter, so access refuses grants beyond their permissions
        const { subject, now } = actor;
        const consenter = Caller.acting(Rollout.consenter(target), subject);

        // ask consent for its requests on other packages' types, its own types going by their policies
        const set = Rollout.permissions(target.installation, description, "installation");
        const installed = principal.installation.reference(target.scope, target.id);
        const requested = set.space ?? [];
        const consented = requested.filter(
            (permission) => permission.packageId !== target.installation.packageId,
        );

        // bind the consented permissions in the role the consenter manages
        const manager = {
            installationId: target.id,
            packageId: target.installation.packageId,
            name: GRANT_NAME,
        };
        const binding = role.call("upsertBinding", {
            ...role.in(target.scope),
            name: `installation/${target.id}`,
            description: `What ${target.installation.name} requests in this space`,
            permissions: consented,
            subject: installed,
            manager,
        });

        // grant the requested permissions outside the space, the universe's in every scope
        const [link] = await Scope.read(reader, [target.scope]);
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
        const granting = await Rollout.grant(reader, installed, [...permissions, ...asking], now);

        // grant as the space, whose authority the grant lends
        const delegator = principal.space.reference(Scope.universe.id, target.scope);
        const lending = Caller.acting(delegator, subject);

        return [
            { caller: AccessContext.of(consenter, now), calls: [binding] },
            ...(granting.length === 0
                ? []
                : [{ caller: AccessContext.of(lending, now), calls: granting }]),
        ];
    },

    /** Build the calls relating a residency home's declared installation, or unrelating it. */
    async relate(
        reader: Pick<Snapshot, "read">,
        target: InstallationRollout,
        set: PermissionSet,
    ): Promise<Call[]> {
        // relate only the installations its stack manages, and only those of a residency's home
        if (!Manager.isManaged(target.installation)) {
            return [];
        }
        const [homed] = await reader
            .read(residency.table)
            .findMany({ where: { space: target.scope }, limit: 1 });
        if (homed === undefined) {
            return [];
        }

        // unrelate one asking for nothing there
        const at = residency.target({ scope: Scope.universe.id, id: homed.id });
        const { id: installed } = target;
        const { name, packageId, selection } = target.installation;
        if ((set.universe ?? []).length === 0) {
            return [residency.call("revoke", { ...at, installation: installed, name })];
        }
        // refuse one following no release, which every installation a residency's home declares follows
        else if (selection.kind !== "release") {
            throw StackError.of(
                "RELEASE_REQUIRED",
                `residency home installation ${name} follows no release`,
            );
        }
        // relate one following a release for what its universe-scoped requests ask
        else {
            const release = selection.version;

            return [
                residency.call("touch", {
                    ...at,
                    installation: installed,
                    name,
                    packageId,
                    release,
                }),
            ];
        }
    },

    /** Build the calls giving an installation its space's one grant of exactly some permissions. */
    async grant(
        reader: Pick<Snapshot, "read">,
        installed: ReturnType<typeof principal.installation.reference>,
        permissions: readonly GrantPermission[],
        now: number,
    ): Promise<Call[]> {
        // leave a grant giving exactly the permissions
        const delegator = principal.space.reference(Scope.universe.id, installed.scope);
        const current = await Grant.find(reader, { delegator, delegate: installed }, now);
        if (
            current !== undefined &&
            canonicalize(current.permissions) === canonicalize(permissions)
        ) {
            return [];
        }

        // replace the outdated grant with the exact one, or revoke it once nothing is requested
        const create = grant.call("create", {
            ...grant.in(installed.scope),
            delegate: installed,
            permissions: [...permissions],
        });
        const revoke = current === undefined ? [] : [grant.call("revoke", grant.target(current))];

        return permissions.length === 0 ? revoke : [create];
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
            .where(and(eq(rollout.table.id, id), Space.served(), launchable()));

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
                `installation ${installed.installation} in ${installed.spaceId} is not served yet`,
            );
        }
        // refuse an installation this machine does not have
        else {
            throw StackError.of(
                "INSTALLATION_NOT_HERE",
                `this machine does not serve installation ${installed.installation} in ${installed.spaceId}`,
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
                eq(installationRevision.table.id, rollout.table.revisionId),
            )
            .where(and(rollout.inScope(spaceId), launchable()))
            .orderBy(installation.table.createdAt, installation.table.id);

        return rows.map((row) => ({
            rollout: Rollout.join(row.rollout, row.installation),
            revision: row.revision,
        }));
    },

    /** Target each installation's selected build in one transaction, described before it. */
    async target(
        reconciliation: ObjectReconciliation,
        rows: readonly InstallationRollout[],
        options: RevisionOptions,
    ): Promise<void> {
        // read the revisions the installations target and their builds
        const { database, store } = reconciliation;
        const snapshot = Snapshot.live(database);
        const targetedIds = rows.flatMap((row) =>
            row.revisionId === null ? [] : [row.revisionId],
        );
        const targeted = await snapshot
            .read(installationRevision.table)
            .findMany({ where: { id: { in: targetedIds } } });
        const built = (
            await snapshot
                .read(build.table)
                .findMany({ where: { id: { in: targeted.map((each) => each.buildId) } } })
        ).map((each) => Build.of(each));

        // describe the revision of each one whose target is not its selected build, before the transaction
        const { crypto } = store.environment;
        const changed: {
            readonly row: InstallationRollout;
            readonly selected: SelectedBuild;
            readonly digest: Digest;
            readonly described: Pick<InstallationRevision, "dependencies" | "states">;
        }[] = [];
        for (const row of rows) {
            // leave one whose selected build is not built yet, or whose target is already it
            const revision = targeted.find((each) => each.id === row.revisionId);
            const current = built.find((each) => each.id === revision?.buildId);
            const selected = await selectBuild(reconciliation, row, current, options);
            if (selected === undefined) {
                continue;
            }
            const digest = await InstallationRevision.digest(selected.manifest, undefined, crypto);
            if (revision?.digest === digest) {
                continue;
            }

            // describe its revision from its build
            const reader = await options.builds.contents(selected.manifest);
            const described = await InstallationRevision.describe(
                row.installation,
                reader,
                store.objects,
                options,
            );
            changed.push({ row, selected, digest, described });
        }
        if (changed.length === 0) {
            return;
        }

        // record each missing revision and target each revision in one transaction
        const calls: Call[] = [];
        for (const { row, selected, digest, described } of changed) {
            // retain the revision of the build once, and target it
            const revisionId = await InstallationRevision.id(row.id, digest, crypto);
            calls.push(
                await InstallationRevision.create(
                    row.installation,
                    { id: selected.build, manifest: selected.manifest },
                    described,
                    undefined,
                    crypto,
                ),
            );
            calls.push(rollout.call("target", { ...rollout.target(row), revisionId }));
        }
        await reconciliation.mutate(calls);
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
            throw StackError.of("REVISION_NOT_FOUND", `revision ${revisionId} not found`);
        }

        // read its build from the store
        const built = await Build.require(snapshot, revision.scope, revision.buildId);

        return packages.contents(built.manifest);
    },

    /** Read the revision an installation follows, absent before its first. */
    async followed(
        database: DatabaseConnection,
        target: Pick<Rollout, "revisionId">,
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
    digest(
        manifest: Digest,
        evaluation: Submission["evaluation"] | undefined,
        crypto: DigestCrypto,
    ): Promise<Digest> {
        return Digest.json(
            evaluation === undefined
                ? { manifest }
                : {
                      manifest,
                      parameters: evaluation.parameters,
                      definition: evaluation.definition,
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

    /** Describe a build's revision of an installation: its dependencies and required states. */
    async describe(
        target: Pick<Installation, "name" | "capabilities" | "allowed">,
        reader: BuildReader,
        objects: readonly ObjectType[],
        options: Pick<RevisionOptions, "builds" | "registry" | "blobs">,
    ): Promise<Pick<InstallationRevision, "dependencies" | "states">> {
        // read the installable packages it depends on, at the releases it compiled against
        const { builds, registry, blobs } = options;
        const read = (dependency: PackageId, manifest: Digest) =>
            builds.pull(registry, dependency, manifest);
        const dependencies = await installableReleases(reader, read);

        // describe the state each declaration requires: main's tables, an installation's pinned release, a resource's own
        const required = new Map<string, JsonValue>();
        const tables = await mainTables(target, reader, { dependencies }, objects, options);
        if (tables !== undefined) {
            required.set(MAIN_DATABASE, tables);
        }
        for (const [name, state] of await declaredStates(reader)) {
            required.set(name, state);
        }

        // write each state as a blob, keyed by its declaration
        const states = await Promise.all(
            [...required].map(
                async ([name, state]) => [name, await blobs.writeJson(state)] as const,
            ),
        );

        return { dependencies, states: Object.fromEntries(states) };
    },

    /** Describe the call that records a space's build and its stack evaluation as a revision. */
    async create(
        target: Pick<Installation, "id" | "scope" | "packageId">,
        evaluated: { readonly id: Identifier<"build">; readonly manifest: Digest },
        described: Pick<InstallationRevision, "dependencies" | "states">,
        evaluation: Submission["evaluation"] | undefined,
        crypto: DigestCrypto,
    ) {
        return installationRevision.call("create", {
            ...installationRevision.in(target.scope),
            installationId: target.id,
            packageId: target.packageId,
            buildId: evaluated.id,
            dependencies: described.dependencies,
            states: described.states,
            ...(evaluation === undefined ? {} : { definition: evaluation.definition }),
            digest: await InstallationRevision.digest(evaluated.manifest, evaluation, crypto),
        });
    },
};

/** Match the installations a machine launches: enabled, and their rollouts not being deleted. */
function launchable(): SQL {
    return and(eq(installation.table.status, "enabled"), isNull(installation.table.deletedAt));
}

/** Join a rollout's installation, which shares its identifier and scope. */
function joinedInstallation(): SQL {
    return and(
        eq(installation.table.id, rollout.table.id),
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

/** Resolve a selection to its build now, recording a pulled release, none until built. */
async function selectBuild(
    reconciliation: ObjectReconciliation,
    row: InstallationRollout,
    current: Build | undefined,
    options: RevisionOptions,
): Promise<SelectedBuild | undefined> {
    // resolve a release or tag to its version, keeping the targeted release build of that version
    const { selection, packageId } = row.installation;
    const snapshot = Snapshot.live(reconciliation.database);
    if (selection.kind === "release" || selection.kind === "tag") {
        const version = await selectedVersion(reconciliation, row, current, options);
        if (version === undefined) {
            return undefined;
        }
        const source = current?.source;
        if (source?.kind === "release" && source.version === version && current !== undefined) {
            return { manifest: Build.manifest(current), source, build: current.id };
        }

        // pull the release and record it as the space's build once
        const { builds, registry } = options;
        const manifest =
            selection.kind === "release" && selection.manifest !== undefined
                ? selection.manifest
                : await registry.find(packageId, version);
        await builds.pull(registry, packageId, manifest);
        const released = { kind: "release" as const, package: packageId, version, manifest };
        const recorded = await Build.release(
            snapshot,
            (calls) => reconciliation.mutate(calls),
            row.scope,
            released,
        );

        return { build: recorded.id, manifest, source: released };
    }
    // take the succeeded build of the commit a repository reference points at
    else if (selection.kind === "repository") {
        const { forge } = options;
        const commit =
            forge === null
                ? undefined
                : await forge.commit(
                      reconciliation.database,
                      selection.repository,
                      selection.reference,
                  );
        if (commit === undefined) {
            return undefined;
        }
        const source = {
            kind: "commit" as const,
            repository: selection.repository,
            commit,
            directory: selection.directory,
        };
        const found = await Build.find(snapshot, row.scope, source);

        return found?.state === "succeeded"
            ? { manifest: Build.manifest(found), source, build: found.id }
            : undefined;
    }
    // take the checkout's newest succeeded build
    else {
        const newest = (
            await snapshot.read(build.table).findMany({
                where: { scope: row.scope, state: "succeeded", expiredAt: { isNull: true } },
            })
        )
            .map((entry) => Build.of(entry))
            .filter(
                (entry) =>
                    entry.source.kind === "checkout" &&
                    entry.source.machine === selection.machine &&
                    entry.source.checkout === selection.checkout &&
                    entry.source.directory === selection.directory,
            )
            .toSorted((left, right) => right.createdAt - left.createdAt)
            .at(0);

        return newest === undefined
            ? undefined
            : { manifest: Build.manifest(newest), source: newest.source, build: newest.id };
    }
}

/** Read the version a release or tag selects now, none before a followed tag replicates. */
async function selectedVersion(
    reconciliation: ObjectReconciliation,
    row: InstallationRollout,
    current: Build | undefined,
    options: RevisionOptions,
): Promise<string | undefined> {
    // take a release's version
    const { selection, packageId } = row.installation;
    const { forge, builds, registry } = options;
    if (selection.kind === "release") {
        return selection.version;
    }
    // take the version a tag points at in the forge's replica
    else if (selection.kind === "tag" && forge !== null) {
        return await forge.version(reconciliation.database, packageId, selection.tag);
    }
    // keep the targeted release of a tag on a machine following no forge
    else if (selection.kind === "tag" && current?.source.kind === "release") {
        return current.source.version;
    }
    // resolve a tag at the registry once, on a machine following no forge
    else if (selection.kind === "tag") {
        const manifest = await registry.find(packageId, selection.tag);

        return (await builds.pull(registry, packageId, manifest)).manifest.package.version;
    }
    // follow nothing else here
    else {
        return undefined;
    }
}

/** Describe the tables a build's types keep in the space's main database, none without types. */
async function mainTables(
    target: Pick<Installation, "name" | "capabilities" | "allowed">,
    reader: BuildReader,
    revision: Pick<InstallationRevision, "dependencies">,
    objects: readonly ObjectType[],
    options: Pick<RevisionOptions, "builds" | "registry">,
): Promise<DatabaseState | undefined> {
    // declare no tables for a package declaring no object types
    const packageId = PackageManifest.package(reader.manifest).id;
    const owned = ObjectDescription.installed(await reader.declarations());
    if (owned.length === 0) {
        return undefined;
    }

    // read the types the builds of the packages it needs declare, pulling none of a package the pool knows
    const pooled = ObjectStore.known(objects, Object.values(accountPackage.objects));
    const known = new Set(objects.map((type) => ObjectTypeReference.key(type.typeReference)));
    const pooledPackages = new Set(pooled.map((type) => type.package.id));
    const description = await PackageDescription.of(reader);
    const needed = new Set(
        Rollout.needed(target, revision, description).map((entry) => entry.package.id),
    );
    const depended = await Promise.all(
        revision.dependencies
            .filter(
                (dependency) =>
                    needed.has(dependency.package.id) && !pooledPackages.has(dependency.package.id),
            )
            .map(async (dependency) => {
                const { id } = dependency.package;
                const pulled = await options.builds.pull(options.registry, id, dependency.manifest);

                return ObjectDescription.installed(await pulled.declarations());
            }),
    );
    const unserved = depended.flat().filter((each) => !known.has(ObjectDescription.key(each)));

    // declare the package's own types against the served ones and those of its dependencies
    const declared = ObjectTypePool.of(pooled).declare([...unserved, ...owned]);

    // describe its durable objects' tables, placing the other packages' they refer to
    const others = objects.filter((type) => type.package.id !== packageId);
    const tables = new Set(
        [...others, ...declared]
            .filter((type) => type.storage === "durable")
            .flatMap((type) => type.tables),
    );

    return DatabaseState.of([...tables], { package: packageId });
}

/** Describe the state each resource declaration of a build requires of its target, by name. */
async function declaredStates(reader: BuildReader): Promise<Map<string, JsonValue>> {
    // read the build's resource declarations and its exact dependencies once an installation of another package is declared
    const own = PackageManifest.package(reader.manifest);
    const resources = (await reader.declarations())
        .filter((declaration) => declaration.kind === RESOURCE_DECLARATION)
        .map((declaration) =>
            ResourceDescription.catchall(schema.json()).parse(declaration.description),
        );
    const callees = resources
        .filter((resource) => resource.kind === InstallationKind.name)
        .map((resource) => InstallationBindingSpec.parse(resource.spec).package);
    const dependencies = callees.some((callee) => callee !== own.id)
        ? Object.values(await reader.dependencies())
        : [];

    // pin an installation to the release of its package the reader compiled against, else take a resource's own state
    const states = new Map<string, JsonValue>();
    for (const { name, kind, spec, ...state } of resources) {
        // pin an installation's callee release
        if (kind === InstallationKind.name) {
            const callee = InstallationBindingSpec.parse(spec).package;
            const release =
                callee === own.id
                    ? own.version
                    : dependencies.find(
                          (entry) => "id" in entry.package && entry.package.id === callee,
                      )?.package.version;
            if (release === undefined) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `the reader binds installation ${name} of ${callee} without depending on it`,
                );
            }
            const pinned: InstallationBindingState = { release: Version.parse(release) };
            states.set(name, pinned);
        }
        // take the state another resource requires, none for a stateless one
        else if (Object.keys(state).length > 0) {
            states.set(name, state);
        }
    }

    return states;
}

import { RequestedPermission } from "@destack/account/authentication";
import { ViewPath } from "@destack/view/declare";
import {
    AccessContext,
    Caller,
    none,
    PermissionReference,
    principal,
    self,
    type GrantDeclaration,
    type GrantContext,
    PermissionSet,
} from "@destack/access";
import {
    and,
    type DatabaseConnection,
    eq,
    inArray,
    isNotNull,
    isNull,
    Snapshot,
    type Select,
    type SQL,
} from "@destack/db";
import {
    changeRequest,
    type Condition,
    controlled,
    defineObject,
    field,
    InstallationBindings,
    InstallationName,
    ObjectDescription,
    ObjectController,
    Observation,
    type ObservationReport,
    SYNCED,
    SYNCED_CONDITION,
    type ObjectType,
    PackageDescription,
    PackageDirectory,
    propagated,
    shareable,
} from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import {
    ComputeDefinition,
    DeclarationName,
    DirectoryGrant,
    PackageId,
    PackageVersion,
    Sandbox,
} from "@destack/package";
import { ViewDescription } from "@destack/package/manifest";
import { Plan, ResourceDescription } from "@destack/resource";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import {
    ObjectReference,
    ObjectTypeReference,
    RequestId,
    Scope,
    type Subject,
} from "@destack/sync";
import { space, Space } from "@destack/account/object";
import { StackError } from "../error/index.ts";
import { InstallationWait } from "./wait.ts";
import { Failure } from "@destack/error";
import { InstallationRevision, installationRevision } from "./revision.ts";
import { argument, plural, t } from "@destack/locale";

/** The name of the installation applying a space's stack from its creation. */
export const STACK_INSTALLATION = "stack";

/** The milliseconds a caller waits for an installation not served yet: a deploy takes seconds. */
const NOT_SERVED_RETRY_MILLISECONDS = 1000;

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
    /** The digest of the image of the build serving the view. */
    image: Digest,
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

/** A package installed in a space at a selection, under a name. */
export const installation = defineObject(
    {
        name: "installation",
        plural: "installations",
        title: t`${plural(argument("count"), { one: "Installation", other: "Installations" })}`,
        scope: [principal.space],
        key: "name",
        fields: {
            /** The installed package. */
            packageId: field.string(PackageId),
            /** The tag, release, reference or checkout it follows, absent for the host's. */
            follows: field.object().optional(),
            /** The package directory within a followed repository or checkout. */
            directory: field.string(PackageDirectory).optional(),
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

            /** The principal submitting the targeted revision, absent before the first one. */
            submittedBy: field.subject().optional(),
            /** The digest of the space's stack the installation last followed, absent for none. */
            spaceStack: field.string(Digest).optional(),
            /** The revision it targets, absent before the first. */
            targetRevision: field.reference(() => installationRevision).optional(),
            /** The revision fully applied, absent before the first application. */
            appliedRevision: field.reference(() => installationRevision).optional(),
            /** The revision the space serves, absent before it serves one. */
            servedRevision: field.reference(() => installationRevision).optional(),
            /** The digest of the objects the installation last bound as their consumer. */
            bound: field.string(Digest).optional(),
            /** The object type a waiting installation waits for, by its reference key. */
            waitType: field.string().optional(),
            /** The condition one of that type's rows meets once it reconciles again. */
            waitCondition: field.json(InstallationWait.condition).optional(),
        },
        // ask approval of a waiting plan through a change request on the installation
        attachments: [changeRequest.attach({ by: "apply" })],
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
            targetRevisionInstallation: {
                kind: "foreignKey",
                on: ["id", "targetRevisionId"],
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
        access: {
            represents: principal.installation,
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
                apply: none(),
                open: none(),
            },
            reserved: ["replicate", "run"],
        },
        traits: [
            // let its controller reconcile it, apply its plans once approved, and finalize an uninstall
            controlled({ approval: true }),
            // propagate each installation into the scopes inside its scope, read there by permission
            propagated({}),
            // bind roles on an installation directly
            shareable({ by: "update" }),
        ],
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("install", {
            fields: [
                "packageId",
                "name",
                "followsPackageId",
                "followsType",
                "followsScope",
                "followsId",
                "directory",
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
                "followsPackageId",
                "followsType",
                "followsScope",
                "followsId",
                "directory",
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

        /** Allow one of the package's optional capabilities, as the installation asked for it. */
        allow: method.mutation({
            title: t`Allow`,
            permission: "allow",
            input: schema.object({ capability: DeclarationName }),
        }),
        /** Withdraw an optional capability allowed at the installation's request. */
        disallow: method.mutation({
            title: t`Disallow`,
            permission: "allow",
            input: schema.object({ capability: DeclarationName }),
        }),

        /** Read the plan waiting for approval with the digest that approves it. */
        plan: method.query({
            title: t`Plan`,
            permission: "read",
            output: schema.object({
                /** The steps waiting, across the installation, those it manages and resources. */
                plan: Plan,
                /** The plan's digest, which an approval gives. */
                digest: Digest,
                /** The approval its change request asks of the space managers, null until open. */
                approval: schema.identifier("approval").nullable(),
            }),
        }),

        /** Target the revision of what the installation follows and apply what its plan allows. */
        reconcile: method.mutation({
            title: t`Reconcile`,
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

/** A persisted installation record. */
export type Installation = Select<typeof installation.table>;

/** Which installations a read takes: the launchable ones, or those served now. */
export type InstallationState = "launchable" | "served";

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

/** The installations of a space: reads, access, relations and releases. */
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

    /** Describe an installation's condition waiting for a reason. */
    waiting(reason: WaitingReason, message: string): Condition {
        return { status: "false", reason, message };
    },

    /** Describe an installation's condition waiting for an image's builds this machine lacks. */
    waitingForBuild(image: Digest): Condition {
        const message = `waiting for build ${image}, which this machine does not have yet`;

        return Installation.waiting("WaitingForBuild", message);
    },

    /** Report whether one following the host's release runs an image this machine does not. */
    isSuperseded(
        installed: Pick<Installation, "followsId">,
        image: Digest,
        trusted: ReadonlySet<Digest>,
    ): boolean {
        return installed.followsId === null && !trusted.has(image);
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

    /** List the packages an installation needs installed, its enabled capabilities' included. */
    needed(
        row: Pick<Installation, "name" | "capabilities" | "allowed">,
        revision: Pick<InstallationRevision, "dependencies">,
        description: PackageDescription | undefined,
    ): { readonly package: PackageVersion; readonly capability?: string }[] {
        // leave out the dependencies that only optional capabilities need
        const optional = new Set(PackageDescription.optionalPackages(description));
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
            ...Installation.enabled(row, description).map(
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
            : PackageDescription.asks(
                  description,
                  Installation.enabled(row, description),
                  delegator,
              );
    },

    /** Record what a space serves of each installation, answering the refusals it records. */
    async serve(
        store: Pick<ObjectStore, "database" | "mutate" | "principal" | "clock" | "environment">,
        spaceId: Identifier<"space">,
        serving: ReadonlyMap<Identifier<"installation">, Serving>,
    ): Promise<Map<Identifier<"installation">, Error>> {
        // read the space's installations
        const snapshot = Snapshot.live(store.database);
        const rows = await snapshot
            .read(installation.table)
            .findMany({ where: { scope: spaceId } });

        // report each one's served revision and Serving, and Synced for a refusal or after one
        const refusals = new Map<Identifier<"installation">, Error>();
        const calls = rows.flatMap((row) => {
            // describe what the space serves of it
            const outcome = serving.get(row.id);
            const served =
                outcome !== undefined && "revisionId" in outcome ? outcome.revisionId : null;
            const refusal = outcome === undefined ? undefined : Installation.refusal(row, outcome);
            const refused = Installation.servingFailure(row);
            const earlier =
                refused === undefined
                    ? undefined
                    : ObjectController.failedCondition(
                          Failure.of(Installation.blocked(row, refused.message)),
                      );
            const isCleared =
                refusal === undefined && earlier?.message === row.conditions[SYNCED]?.message;

            // report Synced failed for a refusal, and synced again once a refused one is served
            const report = {
                conditions: {
                    ...(outcome === undefined ? {} : Installation.serving(outcome).conditions),
                    ...(refusal === undefined
                        ? {}
                        : { [SYNCED]: ObjectController.failedCondition(Failure.of(refusal)) }),
                    ...(isCleared ? { [SYNCED]: SYNCED_CONDITION } : {}),
                },
            };
            const isRecorded =
                row.servedRevisionId === served && Observation.isRecorded(row, report);
            if (!isRecorded && refusal !== undefined) {
                refusals.set(row.id, refusal);
            }

            return isRecorded
                ? []
                : [
                      Observation.call(installation, row, {
                          ...report,
                          fields: { servedRevisionId: served },
                      }),
                  ];
        });
        const context = AccessContext.of(Caller.of(store.principal), store.clock.now());
        if (calls.length > 0) {
            await store.mutate(context, { id: RequestId.create(store.environment), calls });
        }

        return refusals;
    },

    /** Describe a space refusing to serve an installation's targeted revision. */
    blocked(installed: Pick<Installation, "targetRevisionId">, message: string): Error {
        return StackError.of(
            "SERVING_BLOCKED",
            `cannot serve revision '${installed.targetRevisionId}': ${message}`,
        );
    },

    /** Describe a serving outcome as the refusal it is, absent for a served or waiting one. */
    refusal(
        installed: Pick<Installation, "targetRevisionId">,
        outcome: Serving,
    ): Error | undefined {
        return "revisionId" in outcome || outcome.reason === "WaitingForTypes"
            ? undefined
            : Installation.blocked(installed, outcome.message);
    },

    /** Report whether a space applied and serves the revision an installation targets. */
    isServed(
        row: Pick<Installation, "targetRevisionId" | "appliedRevisionId" | "servedRevisionId">,
    ): boolean {
        return (
            row.targetRevisionId !== null &&
            row.appliedRevisionId === row.targetRevisionId &&
            row.servedRevisionId === row.targetRevisionId
        );
    },

    /** Read the space an installation is installed in. */
    space(installed: Pick<Installation, "scope">): Identifier<"space"> {
        return schema.identifier("space").parse(installed.scope);
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

    /** Read who consents to an installation's requests: its submitter, else its installer. */
    consenter(target: Installation): Subject {
        return target.submittedById === null
            ? ObjectReference.at("createdBy", target)
            : ObjectReference.at("submittedBy", target);
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
    ): Promise<Installation> {
        // take the installation served in the call's space, by its identifier or its name
        const spaceId = schema.identifier("space").parse(installed.spaceId);
        const served = await Installation.find(database, spaceId, installed.installation, "served");
        if (served !== undefined) {
            return served;
        }
        const id = await identify(database, spaceId, installed.installation);

        // answer a moving space's installation with where it moves
        const moved =
            served === undefined && id !== undefined
                ? await Installation.moved(database, id, spaceId)
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
        // refuse a suspended installation at once, as it serves again only once enabled
        else if (id !== undefined && (await isSuspended(database, spaceId, id))) {
            throw StackError.of(
                "INSTALLATION_BLOCKED",
                `cannot reach installation '${installed.installation}': it is suspended`,
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

    /** Read the one installation a machine serves of a package, or of one serving its protocols. */
    async requireServing(
        database: DatabaseConnection,
        serving: {
            readonly spaceId: Identifier<"space">;
            readonly packageId: PackageId;
            /** The packages serving it: itself and each serving its protocols. */
            readonly packages: readonly PackageId[];
        },
    ): Promise<Installation> {
        // list the installations of the packages serving it, served, suspended or not
        const { spaceId, packageId, packages } = serving;
        const installed = await database
            .select({ id: installation.table.id, name: installation.table.name })
            .from(installation.table)
            .where(
                and(
                    installation.inScope(spaceId),
                    isNull(installation.table.deletedAt),
                    inArray(installation.table.packageId, [...packages]),
                ),
            );
        const [only] = installed;

        // let a caller wait while the space's stack may still install one, else refuse none
        if (only === undefined && (await isStackPending(database, spaceId))) {
            throw StackError.of(
                "INSTALLATION_UNAVAILABLE",
                `installation serving package '${packageId}' is unavailable: this space does not serve it yet`,
                { data: { retryAfter: NOT_SERVED_RETRY_MILLISECONDS } },
            );
        } else if (only === undefined) {
            throw StackError.of(
                "INSTALLATION_NOT_FOUND",
                `installation serving package '${packageId}' in this space not found`,
            );
        }
        // refuse several, which callers reach by their names
        else if (installed.length > 1) {
            const names = installed.map((each) => each.name).join("', '");
            throw StackError.of(
                "INSTALLATION_CONFLICT",
                `cannot route package '${packageId}': installations '${names}' each serve it`,
            );
        }

        return await Installation.requireServed(database, { spaceId, installation: only.id });
    },

    /** Find an installation of a space in a state, by its identifier or its name. */
    async find(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
        reference: string,
        state: InstallationState,
    ): Promise<Installation | undefined> {
        // match the identifier as given, else the name
        const id = schema.identifier("installation").safeParse(reference);
        const keyed = id.success
            ? eq(installation.table.id, id.data)
            : eq(installation.table.name, reference);
        const [found] = await select(database, [installation.inScope(spaceId), keyed], state);

        return found;
    },

    /** List a space's installations in a state, oldest first. */
    list(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
        state: InstallationState,
    ): Promise<Installation[]> {
        return select(database, [installation.inScope(spaceId)], state);
    },
};

/** Report whether an installation of a space is suspended. */
async function isSuspended(
    database: DatabaseConnection,
    spaceId: Identifier<"space">,
    id: Identifier<"installation">,
): Promise<boolean> {
    const [found] = await database
        .select({ status: installation.table.status })
        .from(installation.table)
        .where(and(installation.inScope(spaceId), eq(installation.table.id, id)));

    return found?.status === "suspended";
}

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

/** Select the installations a machine launches meeting some conditions in a state, oldest first. */
async function select(
    database: DatabaseConnection,
    conditions: readonly SQL[],
    state: InstallationState,
): Promise<Installation[]> {
    // take the enabled ones not being deleted, and of those the ones a machine serves now when asked
    const served =
        state === "served" ? [isNotNull(installation.table.servedRevisionId), Space.served()] : [];
    const rows = await database
        .select({ installation: installation.table })
        .from(installation.table)
        .innerJoin(space.table, eq(space.table.id, installation.table.scope))
        .where(
            and(
                eq(installation.table.status, "enabled"),
                isNull(installation.table.deletedAt),
                ...conditions,
                ...served,
            ),
        )
        .orderBy(installation.table.createdAt, installation.table.id);

    return rows.map((row) => row.installation);
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

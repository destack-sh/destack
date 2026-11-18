import { SERVER_EXPORT } from "@destack/object";
import { checkout } from "../src/computer/checkout/object/index.ts";
import { TestRegistry } from "../src/test/build.ts";
import type { ObjectStore } from "@destack/object/server";
import { BucketKind } from "@destack/bucket/declare";
import { and, DatabaseKind } from "@destack/db";
import { defineKind, type Kind, type Provisioning } from "@destack/package";
import { InstallationKind } from "@destack/object";
import settingPackage from "@destack/setting/package";
import { testHost } from "@destack/test/host";
import type { Crypto } from "@destack/environment";
import objectHandle from "@destack/object/package";
import {
    approval,
    type BindingTarget,
    changeRequest,
    ChangeRequest,
    defineObject,
    field,
    type Field,
    type ObjectDefinition,
    ObjectDescription,
    InstallationBindingSpec,
    installation,
    Observation,
} from "@destack/object";
import { MemoryBuild } from "@destack/package/test";
import { testIdentity } from "@destack/object/test";
import { Plan } from "@destack/resource";
import { ObjectReference, Scope, RequestId } from "@destack/sync";
import { expect, refusal, single, test } from "@destack/test";
import {
    type DatabaseConnection,
    type Select,
    DatabaseState,
    eq,
    Expression,
    isNotNull,
    Snapshot,
} from "@destack/db";
import { Digest, nth, Identifier, present, schema, type VersionRange } from "@destack/schema";
import {
    BROWSER_SANDBOX,
    HOST_SANDBOX,
    Sandbox,
    type DirectoryGrant,
    PackageVersion,
    PackageId,
} from "@destack/package";
import { TestEntitlement } from "@destack/account/test";
import {
    AccessContext,
    accessGrant,
    accessRole,
    accessRolePermission,
    Caller,
    type GrantDeclaration,
    principal,
} from "@destack/access";
import { space } from "@destack/account/object";
import type { ServerRuntime } from "@destack/package/runtime";
import { BuildReader } from "@destack/package/manifest";
import * as object from "@destack/stack/object";
import * as accountObject from "@destack/account/object";
import { Approval } from "../src/index.ts";
import { Rollout } from "@destack/stack/object";
import {
    readInstalled,
    refusedCondition,
    fixtureBuild,
    fixtureOutputs,
    ids,
    insertRollout,
    TestRuntime,
    TestSpace,
    workload,
} from "../src/test/index.ts";
import { argument, plural, t } from "@destack/locale";
import { shareable } from "@destack/object";

/** The notes package the scenarios install. */
const notesPackage = PackageVersion.parse({
    id: ids.package,
    name: "@example/notes",
    version: "2026.9.0",
});

/** The package defining object types, resource kinds and triggers. */
const objectPackage = PackageVersion.parse({
    id: objectHandle.id,
    name: "@destack/object",
    version: "2026.9.0",
});

/** The main workload of the notes build. */
const mainWorkload = { ...workload, resources: [] };

/** A package an application depends on, which its space may lack. */
const VAULT = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000f2",
    name: "@acme/vault",
    version: "2026.9.0",
});

/** A declaration of the notes build: a resource of its kind, or another kind's with its definer. */
type Declaration = (
    | {
          /** The provisioned kind of the resource. */
          readonly resource: Kind<schema.Schema, Provisioning>;
      }
    | {
          /** The declaration kind. */
          readonly kind: "package" | "trigger" | "object" | "setting";
          /** The package defining the declaration's kind. */
          readonly definer: PackageVersion;
      }
) & {
    /** The declaration's name. */
    readonly name: string;
    /** The declaration's description, as the definer's inspector records it. */
    readonly description: Record<string, unknown>;
};

/** Vaults, which no provider here serves. */
const VaultKind = defineKind(
    { name: "vault", provisioned: { spec: schema.object({}) } },
    { package: objectPackage },
);

/** Connections, which no server here serves. */
const ConnectionKind = defineKind(
    { name: "connection", provisioned: { spec: schema.object({}), management: "external" } },
    { package: objectPackage },
);

/** Declare the notes package's note type with some fields, as the object package describes it. */
function noteType(
    fields: Readonly<Record<string, Field>>,
    release: { readonly version?: string; readonly convert?: ObjectDefinition["convert"] } = {},
): Declaration {
    // describe the type as the notes package's release, with the release's conversions
    const { version = notesPackage.version, convert } = release;
    const note = defineObject({
        name: "note",
        plural: "notes",
        title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
        scope: space,
        fields,
        traits: [shareable({})],
        ...(convert === undefined ? {} : { convert }),
    });
    const description = { ...ObjectDescription.of(note), package: { ...notesPackage, version } };

    return { kind: "object", definer: objectPackage, name: "note", description };
}

/** The notes package's note type, whose notes have a body. */
const main = noteType({ body: field.string().optional() });

/** The tasks binding, calling an installation of the notes package. */
const tasks: Declaration = {
    resource: InstallationKind,
    name: "tasks",
    description: { name: "tasks", spec: { package: ids.package } },
};

/** Write an installable build of another package: its description, a space type and a server. */
async function dependencyBuild(owner: PackageVersion, crypto: Crypto): Promise<BuildReader> {
    // describe the package and its one type
    const entry = defineObject({
        name: "entry",
        plural: "entries",
        title: t`${plural(argument("count"), { one: "Entry", other: "Entries" })}`,
        scope: space,
        fields: {},
    });
    const entryDescription = { ...ObjectDescription.of(entry), package: owner };
    const described = {
        ...testIdentity,
        objects: {},
        mounts: {},
        protocols: [],
        views: [],
        permissions: {},
        capabilities: {},
        errors: [],
    };

    // write it with a server output
    return await MemoryBuild.of(
        owner,
        [
            {
                kind: "package",
                kindPackageId: objectPackage.id,
                name: "default",
                description: described,
            },
            {
                kind: "object",
                kindPackageId: objectPackage.id,
                name: entryDescription.name,
                description: entryDescription,
            },
        ],
        crypto,
        {
            outputs: fixtureOutputs({
                server: {
                    runtime: "bun",
                    emit: true,
                    workloads: {},
                    views: {},
                    exports: { [SERVER_EXPORT]: { path: "server/index.js", imports: [] } },
                },
            }),
        },
    );
}

/** The notes package's declaration, requesting permissions by scope. */
function requesting(permissions: GrantDeclaration): Declaration {
    return {
        kind: "package",
        definer: objectPackage,
        name: "notes",
        description: {
            ...testIdentity,
            objects: {},
            mounts: {},
            protocols: [],
            views: [],
            permissions,
            capabilities: {},
            errors: [],
        },
    };
}

/** Write the notes build: its workloads in bun and workerd outputs, and a home view. */
async function notesBuild(
    declared: readonly Declaration[],
    crypto: Crypto,
    sandbox: Sandbox = {},
    workloads: readonly string[] = ["main"],
    requires: Readonly<Record<string, string>> = {},
    version: string = notesPackage.version,
): Promise<BuildReader> {
    // make the workload use every declared resource in both server outputs
    const used = {
        ...workload,
        sandbox: Sandbox.select(
            sandbox,
            HOST_SANDBOX.filter((name) => sandbox[name] !== undefined),
        ),
        resources: declared
            .filter((entry) => "resource" in entry)
            .map((entry) => ({
                packageId: ids.package,
                name: entry.name,
            })),
    };
    const browser = BROWSER_SANDBOX.filter((name) => sandbox[name] !== undefined);
    const output = (runtime: string) => ({
        runtime,
        emit: true,
        workloads: Object.fromEntries(workloads.map((name) => [name, used])),
        views:
            browser.length === 0
                ? {}
                : {
                      home: {
                          entrypoint: "view.js",
                          stylesheets: [],
                          sandbox: Sandbox.select(sandbox, browser),
                          presents: [],
                          categories: [],
                          audience: "person",
                          packages: [],
                          render: { mode: "client" },
                      },
                  },
    });

    // declare each resource as its provisioned kind's, and each other declaration as its definer's
    const built = await MemoryBuild.of(
        { ...notesPackage, version },
        declared.map((entry) => ({
            ...("resource" in entry
                ? { ...entry.resource.identity, provisioned: true as const }
                : { kind: entry.kind, kindPackageId: entry.definer.id }),
            module: "src/resource.ts",
            name: entry.name,
            description: entry.description,
        })),
        crypto,
        { outputs: fixtureOutputs({ server: output("bun"), worker: output("workerd") }), requires },
    );

    return built;
}

/** A runtime that counts attempts of a runner that exits before serving. */
class FailingRuntime extends TestRuntime {
    /** The starts attempted. */
    attempts = 0;

    /** Fail to start. */
    override async start(): Promise<void> {
        this.attempts += 1;
        throw new Error("runner exited before serving");
    }
}

/** The checkout the notes installations follow. */
const selection = {
    kind: "checkout" as const,
    machine: ids.machine,
    checkout: schema.identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-000000000008"),
    directory: "." as const,
};

/** Install notes from a checkout at one revision of its build and return the installation. */
async function installNotes(
    fixture: TestSpace,
    id: Identifier<"installation"> = ids.notes,
    build?: BuildReader,
    builds: readonly BuildReader[] = [],
) {
    const { database } = fixture;
    const now = fixture.machine.clock.now();
    const notes = await insertRollout(
        database,
        {
            id,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            follows: checkout.reference(selection.machine, selection.checkout),
            directory: selection.directory,
            installer: fixture.owner.subject,
        },
        now,
    );
    await recordBuild(fixture, build, builds);

    return notes;
}

/** Change an installation's fields as its owner writes them, at its next generation. */
async function changeInstallation(
    database: DatabaseConnection,
    id: Identifier<"installation">,
    fields: Partial<Select<typeof installation.table>>,
): Promise<void> {
    await database.transaction(async (transaction) => {
        const [row] = await transaction
            .select()
            .from(installation.table)
            .where(eq(installation.table.id, id));
        await transaction
            .update(installation.table)
            .set({ ...fields, generation: present(row, `installation ${id}`).generation + 1 })
            .where(eq(installation.table.id, id));
    });
}

/** Record a build of the followed checkout, as its machine does, and let the space deploy it. */
async function recordBuild(
    fixture: TestSpace,
    build?: BuildReader,
    builds: readonly BuildReader[] = [],
) {
    // record it a millisecond after the last, as the newest build of a checkout is the one it follows
    const reader = build ?? (await notesBuild([main], fixture.environment.crypto));
    await fixture.simulation.sleep(1);
    const built = await fixture.recordBuild(reader, undefined, builds);
    await fixture.simulation.idle();

    return built;
}

/** Record a build of the followed checkout and read the revision the space targets for it. */
async function submitRevision(
    fixture: TestSpace,
    installationId: Identifier<"installation">,
    build?: BuildReader,
    builds: readonly BuildReader[] = [],
) {
    // record the checkout's build, which the space deploys
    const built = await recordBuild(fixture, build, builds);

    // read the revision of the build it targets
    const [revision] = await fixture.database
        .select()
        .from(object.installationRevision.table)
        .where(
            and(
                eq(object.installationRevision.table.installationId, installationId),
                eq(object.installationRevision.table.buildId, built.build),
            ),
        );

    return present(revision, `the revision of build ${built.build}`);
}

/** Restart the machine serving the space on some runtimes and options. */
async function serveControllers(
    fixture: TestSpace,
    options: {
        readonly runtimes?: readonly ServerRuntime[];
        readonly runtime?: TestRuntime;
        readonly isEnforcingEntitlements?: boolean;
        readonly destack?: VersionRange;
    } = {},
) {
    // run each runtime the options leave out as a test runtime over the machine's builds
    const builds = await fixture.builds();
    const runtime =
        options.runtime ?? new TestRuntime({ scheduler: fixture.environment.scheduler, builds });
    const names = options.runtimes ?? ["bun"];
    await fixture.restart({
        runtimes: (machine) =>
            names.map((name) =>
                name === runtime.name ? runtime : new TestRuntime({ ...machine, name }),
            ),
        ...(options.destack === undefined ? {} : { destack: options.destack }),
        ...(options.isEnforcingEntitlements === true ? { isEnforcingEntitlements: true } : {}),
    });
    const store = await fixture.store();

    return {
        runtime,
        store,
        authorizer: store.authorizer,
        approve: (id: Identifier<"installation">) => approvePlan(fixture.database, store, id),
        uninstall: (id: Identifier<"installation">) =>
            store.mutate(store.as(object.rollout), {
                id: RequestId.create(fixture.machine.environment),
                calls: [installation.call("delete", installation.target({ scope: ids.space, id }))],
            }),
    };
}

/** Select a target for a notes declaration of an installation, as its stack does. */
async function selectBinding(
    database: DatabaseConnection,
    id: Identifier<"installation">,
    name: string,
    target: BindingTarget,
) {
    // change the installation's desired bindings, moving its generation as its update does
    const [row] = await database
        .select({
            bindings: installation.table.bindings,
            generation: installation.table.generation,
        })
        .from(installation.table)
        .where(eq(installation.table.id, id));
    const { bindings, generation } = present(row, `installation ${id}`);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({
                    bindings: {
                        ...bindings,
                        [ids.package]: { ...bindings[ids.package], [name]: { target } },
                    },
                    generation: generation + 1,
                })
                .where(eq(installation.table.id, id)),
    );
}

/** Read the revision a deployment runs. */
async function revisionOf(database: DatabaseConnection, id: Identifier<"installation-revision">) {
    const [revision] = await database
        .select()
        .from(object.installationRevision.table)
        .where(eq(object.installationRevision.table.id, id));

    return present(revision, `revision ${id}`);
}

/** Read how far an installation is, with its failure or what its ready condition says. */
async function readiness(database: DatabaseConnection, id: Identifier<"installation">) {
    const [row] = await database
        .select({
            generation: installation.table.generation,
            conditions: installation.table.conditions,
        })
        .from(installation.table)
        .where(eq(installation.table.id, id));
    const installed = present(row, "the installation");

    return [Observation.progress(installed), Observation.message(installed)];
}

/** Approve the plan an installation waits on as it stands. */
async function approvePlan(
    database: DatabaseConnection,
    store: ObjectStore,
    id: Identifier<"installation">,
) {
    // read the plan the installation waits on
    const [target] = await Rollout.read(
        Snapshot.live(database),
        await database
            .select()
            .from(object.rollout.table)
            .where(eq(object.rollout.table.installationId, id)),
    );
    const resources = [object.database, object.dependency];
    const pending = await Approval.of(
        Snapshot.live(database),
        resources,
        present(target, "the installation's rollout"),
    );
    const plan = pending.plan();
    const digest = await Plan.digest(plan, database.state.environment.crypto);

    // approve it as the system, answering the approved plan
    await store.mutate(store.as(object.rollout), {
        id: RequestId.create(store.environment),
        calls: [
            object.rollout.call("approve", {
                ...object.rollout.target({
                    scope: present(target, "the installation's rollout").scope,
                    id: present(target, "the installation's rollout").installationId,
                }),
                plan: digest,
            }),
        ],
    });

    return plan;
}

test("retire a suspended application's deployment and instance", async () => {
    // install notes, whose notes the space's main database stores
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const { runtime } = await serveControllers(fixture);
    const notes = await installNotes(fixture);

    // deploy it on a bun machine, apply main, and run its instance on this machine
    await fixture.simulation.idle();
    const deployed = await database.select().from(object.deployment.table);
    await fixture.simulation.idle();
    const running = nth(await database.select().from(object.instance.table), 0);
    const installed = await readInstalled(database, notes.installationId);
    const revision = await revisionOf(database, nth(deployed, 0).revisionId);
    const run = nth(runtime.started, 0);

    // deploy only the bun output and run it without its main database, which its revision leaves out
    expect({
        deployments: deployed.map((entry) => [
            entry.status,
            entry.output,
            entry.runtime,
            entry.workload,
            entry.description,
        ]),
        instance: [running.status, running.machineId],
        run: [run.instanceId, run.deploymentId, run.scope, run.output, run.workload],
        resources: run.resources,
        bindings: revision.bindings,
        applied: installed.appliedRevisionId === installed.targetRevisionId,
    }).toEqual({
        deployments: [["active", "server", "bun", "main", mainWorkload]],
        instance: ["running", fixture.machine.id],
        run: [running.id, nth(deployed, 0).id, ids.space, "server", "main"],
        resources: [],
        bindings: [],
        applied: true,
    });

    // suspend the application
    await changeInstallation(database, notes.installationId, { status: "suspended" });
    await fixture.simulation.idle();
    const retired = nth(await database.select().from(object.deployment.table), 0);
    const stopped = nth(await database.select().from(object.instance.table), 0);
    expect([retired.status, stopped.status, runtime.stopped]).toEqual([
        "retired",
        "stopped",
        [running.id],
    ]);
});

test("retain an uninstalled application's tables for approval", async () => {
    // install notes without a stack and deploy it
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const notes = await installNotes(fixture);

    // provision and apply main, deploy once the space serves the bound revision, and start the instance
    await fixture.simulation.idle();
    const deployed = nth(await database.select().from(object.deployment.table), 0);
    const applied = nth(await database.select().from(object.database.table), 0);
    const running = nth(await database.select().from(object.instance.table), 0);
    const [revision] = await database
        .select({ states: object.installationRevision.table.states })
        .from(object.installationRevision.table)
        .where(eq(object.installationRevision.table.id, deployed.revisionId));
    const required = present(
        present(revision, "the deployed revision").states["main"],
        "main's tables",
    );
    const bound = await (await fixture.space).blobs.readJson(required, DatabaseState);
    expect({
        main: [applied.name, applied.ownerId, applied.conditions["Ready"]?.reason],
        tables: bound.tables.sqlite.map((state) => state.table.name),
        instance: [running.deploymentId, running.status],
    }).toEqual({
        main: ["main", ids.space, "Applied"],
        tables: ["example__notes__note"],
        instance: [deployed.id, "running"],
    });

    // uninstall notes: drain its deployment, remove it, and wait for approval to drop its tables
    await changeInstallation(database, notes.installationId, {
        deletedAt: fixture.machine.clock.now(),
    });
    await fixture.simulation.idle();
    const retained = nth(await database.select().from(object.database.table), 0);
    const remaining = await database
        .select({ name: installation.table.name })
        .from(installation.table)
        .where(isNotNull(installation.table.name))
        .orderBy(installation.table.name);
    expect({
        installations: remaining,
        revisions: await database
            .select({ id: object.installationRevision.table.id })
            .from(object.installationRevision.table)
            .where(eq(object.installationRevision.table.installationId, notes.installationId)),
        main: [
            retained.conditions["Ready"]?.reason,
            retained.plan?.steps.map((step) => [step.action, step.risk, step.target]) ?? null,
        ],
    }).toEqual({
        installations: [{ name: "stack" }],
        revisions: [],
        main: [
            "AwaitingApproval",
            [
                [
                    "delete",
                    "safe",
                    "table/example__notes__note/dependent/destack__access__relationship",
                ],
                ["delete", "destructive", "table/example__notes__note"],
            ],
        ],
    });
});

test("restart a failing instance after a doubling backoff", async () => {
    // deploy notes with an applied database on a simulated machine with a failing runner
    const fixture = await TestSpace.open();
    const { simulation } = fixture;
    const { database } = fixture;
    const failing = new FailingRuntime({
        scheduler: simulation.scheduler,
        builds: await fixture.builds(),
    });
    await serveControllers(fixture, { runtime: failing });
    const { first, early, second, recorded } = await (async () => {
        await installNotes(fixture);
        await fixture.simulation.idle();
        const deployed = nth(await database.select().from(object.deployment.table), 0);

        // fail, keep the instance through its backoff, then restart it once the backoff passed
        const backoff = async () => {
            const [row] = await database
                .select()
                .from(object.instance.table)
                .where(eq(object.instance.table.deploymentId, deployed.id));
            const { id, restartAt, stoppedAt } = present(row, "the deployment's instance");

            return { id, backoff: (restartAt ?? 0) - (stoppedAt ?? 0) };
        };
        await fixture.simulation.idle();
        const failed = await backoff();
        await fixture.simulation.idle();
        const retried = await backoff();
        await fixture.simulation.sleep(retried.backoff);
        const restarted = await backoff();

        return {
            first: failed.backoff,
            early: retried.backoff,
            second: restarted.backoff,
            recorded: await database.select().from(object.instance.table),
        };
    })();
    expect({
        backoffs: [first, early, second],
        attempts: failing.attempts,
        instances: recorded.map((entry) => [
            entry.status,
            entry.restarts,
            entry.conditions["Ready"]?.status,
            entry.conditions["Ready"]?.reason,
            entry.conditions["Ready"]?.message,
        ]),
    }).toEqual({
        backoffs: [1000, 1000, 2000],
        attempts: 2,
        instances: [["failed", 1, "false", "StartFailed", "runner exited before serving"]],
    });
});

test("serve an applied application again after its machine restarts", async () => {
    // run notes with an applied database once its space serves the revision
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const builds = await fixture.builds();
    const before = new TestRuntime({ scheduler: fixture.environment.scheduler, builds });
    await serveControllers(fixture, { runtime: before });
    const notes = await installNotes(fixture);
    await fixture.simulation.idle();
    const applied = await readInstalled(database, notes.installationId);

    // restart the machine with an idle runtime and reconcile again
    const after = new TestRuntime({ scheduler: fixture.environment.scheduler, builds });
    await serveControllers(fixture, { runtime: after });
    await fixture.simulation.idle();
    const restarted = await readInstalled(database, notes.installationId);
    const recorded = await database.select().from(object.instance.table);

    // keep the applied revision and its one deployment, and start its instance again
    expect({
        applied: [applied.appliedRevisionId, applied.installation.conditions["Ready"]?.status],
        restarted: [
            restarted.appliedRevisionId,
            restarted.servedRevisionId,
            restarted.installation.conditions["Ready"]?.status,
        ],
        deployments: (await database.select().from(object.deployment.table)).length,
        before: before.started.map((run) => run.instanceId),
        after: after.started.map((run) => run.instanceId),
        instances: recorded.map((entry) => [entry.id, entry.status]),
    }).toEqual({
        applied: [applied.targetRevisionId, "true"],
        restarted: [applied.targetRevisionId, applied.targetRevisionId, "true"],
        deployments: 1,
        before: [nth(recorded, 0).id],
        after: [nth(recorded, 0).id],
        instances: [[nth(recorded, 0).id, "running"]],
    });
});

test("replace a deployment only once the next revision runs", async () => {
    // run the first revision of notes with its applied database
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const notes = await installNotes(fixture);
    await fixture.simulation.idle();
    const first = nth(await database.select().from(object.deployment.table), 0);
    await fixture.simulation.idle();

    // submit a second revision a millisecond later that deploys next to the first and drains it
    await fixture.simulation.sleep(1);
    const titled = noteType({ body: field.string().optional(), title: field.string().optional() });
    const second = await submitRevision(
        fixture,
        notes.installationId,
        await notesBuild([titled], fixture.environment.crypto),
    );
    await fixture.simulation.idle();
    const replacement = nth(
        await database
            .select()
            .from(object.deployment.table)
            .where(eq(object.deployment.table.revisionId, second.id)),
        0,
    );

    // run the replacement and stop and retire the drained first revision as its instance's changes wake it
    await fixture.simulation.idle();
    const deployed = await database
        .select()
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);
    const recorded = await database
        .select()
        .from(object.instance.table)
        .orderBy(object.instance.table.createdAt, object.instance.table.id);
    const installed = await readInstalled(database, notes.installationId);
    const [stopped, started] = recorded;

    // stop the first instance no earlier than the replacement starts
    expect({
        isServedUntilReplaced:
            present(stopped?.stoppedAt, "the first stop") >=
            present(started?.startedAt, "the replacement's start"),
        deployments: deployed.map((entry) => [
            entry.revisionId,
            entry.output,
            entry.workload,
            entry.status,
        ]),
        instances: recorded.map((entry) => [entry.deploymentId, entry.status, entry.restarts]),
        applied: installed.appliedRevisionId,
    }).toEqual({
        isServedUntilReplaced: true,
        deployments: [
            [first.revisionId, "server", "main", "retired"],
            [second.id, "server", "main", "active"],
        ],
        instances: [
            [first.id, "stopped", 0],
            [replacement.id, "running", 0],
        ],
        applied: second.id,
    });
});

test("redeploy once the machine's runtimes change", async () => {
    // deploy notes on a machine running workerd
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture, { runtimes: ["workerd"] });
    await installNotes(fixture);
    await fixture.simulation.idle();

    // serve it on a bun machine a millisecond later to replace and retire the workerd deployment
    await fixture.simulation.sleep(1);
    await serveControllers(fixture, { runtimes: ["bun"] });
    await fixture.simulation.idle();
    const deployed = await database
        .select()
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);
    expect(deployed.map((entry) => [entry.output, entry.runtime, entry.status])).toEqual([
        ["worker", "workerd", "retired"],
        ["server", "bun", "active"],
    ]);
});

test("redeploy once a captured binding targets another resource", async () => {
    // deploy notes with the bucket it owns for its files
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const files: Declaration = {
        resource: BucketKind,
        name: "files",
        description: { name: "files", spec: {} },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, files], fixture.environment.crypto),
    );
    await fixture.simulation.idle();
    const owned = nth(await database.select().from(object.bucket.table), 0);

    // rebind the files a millisecond later to a second bucket of the same declaration
    await fixture.simulation.sleep(1);
    const now = fixture.machine.clock.now();
    const second = nth(
        await database.transaction(
            async (transaction) =>
                await transaction
                    .insert(object.bucket.table)
                    .values({
                        ...owned,
                        id: Identifier.create("bucket", fixture.environment),
                        name: "notes.archive",
                        managerInstallationId: null,
                        managerPackageId: null,
                        managerName: null,
                        createdAt: now,
                        updatedAt: now,
                    })
                    .returning(),
        ),
        0,
    );
    await selectBinding(database, notes.installationId, "files", { type: "bucket", id: second.id });

    // redeploy with the new target and retire the old deployment
    await fixture.simulation.idle();
    const deployed = await database
        .select()
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);
    const bound = await Promise.all(
        deployed.map(async (entry) => {
            const revision = await revisionOf(database, entry.revisionId);
            const taken = revision.bindings.filter((each) => each.name === "files");

            return [entry.id, nth(taken, 0).target];
        }),
    );
    expect({
        deployments: deployed.map((entry) => entry.status),
        captures: bound,
    }).toEqual({
        deployments: ["retired", "active"],
        captures: [
            [nth(deployed, 0).id, owned.id],
            [nth(deployed, 1).id, second.id],
        ],
    });
});

test("refuse a release built with an unsupported Destack", async () => {
    // deploy notes built with a released Destack on a machine running only a later one, then only an earlier one
    const built = "2026.9.0";
    const deploy = async (destack: VersionRange) => {
        const fixture = await TestSpace.open();
        await serveControllers(fixture, { destack });
        const { crypto } = fixture.environment;
        const build = await fixtureBuild(ids.package, "2026.9.0", crypto);
        const released = new BuildReader(
            { ...build.manifest, destack: built },
            build.stream,
            crypto,
        );
        const notes = await installNotes(fixture, ids.notes, released);
        await fixture.simulation.idle();
        const refused = single(fixture.takeRefusals());
        const installed = await readInstalled(fixture.database, notes.installationId);
        const deployed = await fixture.database.select().from(object.deployment.table);
        const synced = installed.installation.conditions["Synced"];

        return [deployed.length, refused, synced?.message];
    };

    // refuse each, the release too old for the later machine and too new for the earlier one
    const older = `@test/package is built with Destack ${built}, older than this machine supports (2099.1.0)`;
    const newer = `@test/package is built with Destack ${built}, newer than this machine's 2001.1.0`;
    expect({
        later: await deploy({ oldest: "2099.1.0", newest: "2099.1.0" }),
        earlier: await deploy({ oldest: "2001.1.0", newest: "2001.1.0" }),
    }).toEqual({
        later: [
            0,
            ["NOT_IMPLEMENTED", "RELEASE_UNSUPPORTED", older],
            `@destack/stack RELEASE_UNSUPPORTED: ${older}`,
        ],
        earlier: [
            0,
            ["NOT_IMPLEMENTED", "RELEASE_UNSUPPORTED", newer],
            `@destack/stack RELEASE_UNSUPPORTED: ${newer}`,
        ],
    });
});

test("refuse a build with no output for the machine's runtimes", async () => {
    // install notes with its workload only in a bun output
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture, {
        runtimes: ["workerd"],
    });
    const notes = await installNotes(
        fixture,
        ids.notes,
        await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto),
    );

    // refuse deploying it on a workerd machine
    const message = "workloads main have no output on workerd";
    await fixture.simulation.idle();
    const refused = single(fixture.takeRefusals());
    const installed = await readInstalled(database, notes.installationId);
    expect({
        deployments: await database.select().from(object.deployment.table),
        refused,
        synced: installed.installation.conditions["Synced"],
        applied: installed.appliedRevisionId,
    }).toEqual({
        deployments: [],
        refused: ["NOT_IMPLEMENTED", "RELEASE_UNSUPPORTED", message],
        synced: refusedCondition(installed, "RELEASE_UNSUPPORTED", message),
        applied: null,
    });
});

test("observe Ready at each generation of an unchanged revision", async () => {
    // deploy notes, then bump its generation
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const notes = await installNotes(fixture);
    await fixture.simulation.idle();
    const first = await readInstalled(database, notes.installationId);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ generation: first.installation.generation + 1 })
                .where(eq(installation.table.id, notes.installationId)),
    );

    // deploy it again
    await fixture.simulation.idle();
    const next = await readInstalled(database, notes.installationId);
    expect({
        revisions: [first.appliedRevisionId, next.appliedRevisionId],
        ready: [readyOf(first), readyOf(next)],
    }).toEqual({
        revisions: [first.appliedRevisionId, first.appliedRevisionId],
        ready: [
            {
                status: "true",
                reason: "Deployed",
                message: "1 workloads deployed",
                observedGeneration: first.installation.generation,
            },
            {
                status: "true",
                reason: "Deployed",
                message: "1 workloads deployed",
                observedGeneration: first.installation.generation + 1,
            },
        ],
    });
});

test("refuse a build needing a newer builtin than linked", async () => {
    // link a builtin at 2026.9.0, and install notes requiring its 2026.10.0
    const boxes = PackageVersion.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000e2",
        name: "@example/boxes",
        version: "2026.9.0",
    });
    const linked = await MemoryBuild.of(boxes, [], testHost().crypto, { role: "builtin" });
    const fixture = await TestSpace.open({ linked: [linked] });
    const { database } = fixture;
    await serveControllers(fixture);
    const requires = { [boxes.name]: "2026.10.0" };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main], fixture.environment.crypto, {}, ["main"], requires),
    );

    // refuse deploying it
    const message =
        "cannot deploy '@example/notes@2026.9.0': it needs @example/boxes 2026.10.0 or newer, and this machine runs 2026.9.0";
    await fixture.simulation.idle();
    const refused = single(fixture.takeRefusals());
    const installed = await readInstalled(database, notes.installationId);
    expect({
        deployments: await database.select().from(object.deployment.table),
        refused,
        synced: installed.installation.conditions["Synced"],
    }).toEqual({
        deployments: [],
        refused: ["PRECONDITION_FAILED", "RELEASE_BLOCKED", message],
        synced: refusedCondition(installed, "RELEASE_BLOCKED", message),
    });
});

test("refuse a build declaring its own database", async () => {
    // install notes declaring a database in addition to the space's main database
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const archive: Declaration = {
        resource: DatabaseKind,
        name: "archive",
        description: { name: "archive", spec: {} },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, archive], fixture.environment.crypto),
    );

    // refuse deploying it
    const message = `${ids.package} declares the databases archive: the space's main database keeps every installation's tables`;
    await fixture.simulation.idle();
    const refused = single(fixture.takeRefusals());
    const installed = await readInstalled(database, notes.installationId);
    expect({
        deployments: await database.select().from(object.deployment.table),
        refused,
        synced: installed.installation.conditions["Synced"],
        applied: installed.appliedRevisionId,
    }).toEqual({
        deployments: [],
        refused: ["PRECONDITION_FAILED", "DATABASE_BLOCKED", message],
        synced: refusedCondition(installed, "DATABASE_BLOCKED", message),
        applied: null,
    });
});

test("refuse a build routing hooks nothing serves", async () => {
    // install notes routing hooks on its note type, built without stack's hooks workload
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const hooked: Declaration = {
        ...requesting({}),
        description: {
            ...testIdentity,
            objects: {
                notes: {
                    type: { packageId: ids.package, type: "note" },
                    hooks: { create: ["handle"] },
                },
            },
            mounts: {},
            protocols: [],
            views: [],
            permissions: {},
            capabilities: {},
            errors: [],
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, hooked], fixture.environment.crypto),
    );

    // refuse deploying it
    const message = `${notesPackage.name} routes hooks, but nothing serves them: declare @destack/stack`;
    await fixture.simulation.idle();
    const refused = single(fixture.takeRefusals());
    const installed = await readInstalled(database, notes.installationId);
    expect({
        deployments: await database.select().from(object.deployment.table),
        refused,
        synced: installed.installation.conditions["Synced"],
    }).toEqual({
        deployments: [],
        refused: ["PRECONDITION_FAILED", "HOOK_BLOCKED", message],
        synced: refusedCondition(installed, "HOOK_BLOCKED", message),
    });
});

test("refuse a build declaring mounts no workload answers", async () => {
    // install notes declaring a mount, built without any workload
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const adapted: Declaration = {
        ...requesting({}),
        description: {
            ...testIdentity,
            objects: {},
            mounts: { inbox: { path: "/inbox" } },
            protocols: [],
            views: [],
            permissions: {},
            capabilities: {},
            errors: [],
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, adapted], fixture.environment.crypto, {}, []),
    );

    // refuse deploying it
    const message = `${notesPackage.name} declares mounts, but no workload of its build answers them`;
    await fixture.simulation.idle();
    const refused = single(fixture.takeRefusals());
    const installed = await readInstalled(database, notes.installationId);
    expect({
        deployments: await database.select().from(object.deployment.table),
        refused,
        synced: installed.installation.conditions["Synced"],
    }).toEqual({
        deployments: [],
        refused: ["PRECONDITION_FAILED", "MOUNT_BLOCKED", message],
        synced: refusedCondition(installed, "MOUNT_BLOCKED", message),
    });
});

test("run each workload with its allowed capabilities and directories", async () => {
    // install notes whose workload runs git, and whose optional capabilities call an issue tracker, index chosen directories or scan receipts
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const { runtime } = await serveControllers(fixture);
    const base = Sandbox.parse({ run: { commands: ["git"] } });
    const declared = requesting({});
    const request: Declaration = {
        ...declared,
        description: {
            ...declared.description,
            capabilities: {
                issues: {
                    reason: t`Link issues`,
                    optional: true,
                    sandbox: { network: { connect: ["api.github.com"] } },
                },
                index: {
                    reason: t`Index your notes`,
                    optional: true,
                    sandbox: { files: { access: "read" } },
                },
                scanning: { reason: t`Scan receipts`, optional: true, sandbox: { camera: {} } },
            },
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, request], fixture.environment.crypto, base),
    );
    const notesDirectory = { path: "/Users/person/Notes", access: "read" as const };
    const grant = async (
        capabilities: readonly string[],
        directories: readonly DirectoryGrant[] = [],
    ) => {
        await changeInstallation(database, notes.installationId, {
            capabilities: [...capabilities],
            directories: [...directories],
        });

        return fixture.simulation.idle();
    };
    const deployed = async () =>
        (
            await database
                .select()
                .from(object.deployment.table)
                .orderBy(object.deployment.table.createdAt, object.deployment.table.id)
        ).map((entry) => [entry.status, entry.description.sandbox, entry.directories]);

    // refuse allowing an undeclared capability, a directory without fs, writing a directory fs reads,
    //  and a directory reaching the host's service sockets
    const refuse = async (
        message: string,
        capabilities: readonly string[],
        directories: readonly DirectoryGrant[] = [],
    ) => {
        await grant(capabilities, directories);
        fixture.expectRefusals([["PRECONDITION_FAILED", "CAPABILITY_BLOCKED", message]]);

        return await readiness(database, notes.installationId);
    };
    const undeclared = await refuse("notes 2026.9.0 declares no capabilities microphone to allow", [
        "microphone",
    ]);
    const unreadable = await refuse(
        "notes 2026.9.0 enables no fs ability to grant directories /Users/person/Notes",
        [],
        [notesDirectory],
    );
    const overreaching = await refuse(
        "notes 2026.9.0 enables fs for reading only, refusing to grant writing /Users/person/Notes",
        ["index"],
        [{ ...notesDirectory, access: "write" }],
    );
    const socketed = await refuse(
        "notes 2026.9.0 may not be granted directories reaching the host's service sockets in /run, /var/run or all of /tmp: /var/run",
        ["index"],
        [{ path: "/var/run", access: "read" }],
    );
    const refusedDeployments = await deployed();

    // deploy with the build's sandbox alone
    await grant([]);
    await fixture.simulation.idle();
    const unallowed = await deployed();

    // allow the issues, indexing and scanning a millisecond later, and redeploy the workload with their host abilities and the directory
    await fixture.simulation.sleep(1);
    await grant(["issues", "index", "scanning"], [notesDirectory]);
    const allowed = await deployed();
    expect({
        undeclared,
        unreadable,
        overreaching,
        socketed,
        refusedDeployments,
        started: runtime.started.map((started) => [started.sandbox, started.directories]),
        unallowed,
        allowed,
    }).toEqual({
        undeclared: [
            "failed",
            "@destack/stack CAPABILITY_BLOCKED: notes 2026.9.0 declares no capabilities microphone to allow",
        ],
        unreadable: [
            "failed",
            "@destack/stack CAPABILITY_BLOCKED: notes 2026.9.0 enables no fs ability to grant directories /Users/person/Notes",
        ],
        overreaching: [
            "failed",
            "@destack/stack CAPABILITY_BLOCKED: notes 2026.9.0 enables fs for reading only, refusing to grant writing /Users/person/Notes",
        ],
        socketed: [
            "failed",
            "@destack/stack CAPABILITY_BLOCKED: notes 2026.9.0 may not be granted directories reaching the host's service sockets in /run, /var/run or all of /tmp: /var/run",
        ],
        // leave the running deployment as it was through each refused grant
        refusedDeployments: [["active", base, []]],
        started: [
            [base, []],
            [
                { ...base, network: { connect: ["api.github.com"] }, files: { access: "read" } },
                [notesDirectory],
            ],
        ],
        unallowed: [["active", base, []]],
        allowed: [
            ["retired", base, []],
            [
                "active",
                { ...base, network: { connect: ["api.github.com"] }, files: { access: "read" } },
                [notesDirectory],
            ],
        ],
    });
});

test("fail an instance granted files on workerd", async () => {
    // deploy notes requiring files on a machine running workerd only
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture, {
        runtimes: ["workerd"],
        runtime: new TestRuntime({
            scheduler: fixture.environment.scheduler,
            name: "workerd",
            builds: await fixture.builds(),
        }),
    });
    const files = Sandbox.parse({ files: { access: "read" } });
    await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main], fixture.environment.crypto, files),
    );
    await fixture.simulation.idle();

    // record the refusal on the instance
    await fixture.simulation.idle();
    const [recorded] = await database.select().from(object.instance.table);
    expect([
        recorded?.status,
        recorded?.conditions["Ready"]?.reason,
        recorded?.conditions["Ready"]?.message,
    ]).toEqual([
        "failed",
        "StartFailed",
        "filesystem ability is invalid: the 'workerd' runtime has no filesystem to grant",
    ]);
});

test("bind an installation binding once its target has an address", async () => {
    // install notes calling the tasks installation besides its main database bound to a provisioned database
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const { runtime } = await serveControllers(fixture);
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, tasks], fixture.environment.crypto),
    );

    // refuse to deploy while no stack binds the tasks installation to an address
    await fixture.simulation.idle();
    fixture.expectRefusals([
        [
            "PRECONDITION_FAILED",
            "BINDING_BLOCKED",
            `cannot apply installations '${ids.package} tasks': bind them to their addresses first`,
        ],
    ]);
    const refused = await readiness(database, notes.installationId);

    // declare the tasks dependency at the address of the notes installation itself and bind it
    const now = fixture.machine.clock.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const declared = {
        ...record,
        ...ObjectReference.columns(
            "owner",
            principal.space.reference(Scope.universe.id, ids.space),
        ),
        id: schema
            .identifier("dependency")
            .parse("dependency-01996ab0-0000-7000-8000-000000000021"),
        name: "tasks",
        definitionPackageId: objectPackage.id,
        definitionVersion: "2026.9.0",
        definitionName: "tasks",
        spec: { ...InstallationBindingSpec.parse(tasks.description["spec"]), address: "notes" },
        provider: "http",
        reference: "notes",
    };
    await database.transaction(
        async (transaction) => await transaction.insert(object.dependency.table).values(declared),
    );
    await selectBinding(database, notes.installationId, "tasks", {
        type: "dependency",
        id: declared.id,
    });

    // observe it ready as declared and deploy and run with the database of the installation
    await fixture.simulation.idle();
    const observed = nth(
        await database
            .select({ conditions: object.dependency.table.conditions })
            .from(object.dependency.table)
            .where(eq(object.dependency.table.id, declared.id)),
        0,
    );
    await fixture.simulation.idle();
    const declaredRun = present(runtime.started[0], "the run with the declared address");
    const dependency = declaredRun.resources.find((entry) => entry.kind === "installation");

    // bind the tasks to the board installation of another space, by its name there
    const acme = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000031");
    const board = "board";
    await selectBinding(database, notes.installationId, "tasks", {
        type: "installation",
        name: board,
        spaceId: acme,
    });

    // deploy it and run its instance
    await fixture.simulation.idle();
    const deployed = nth(
        await database
            .select()
            .from(object.deployment.table)
            .where(eq(object.deployment.table.status, "active")),
        0,
    );
    await fixture.simulation.idle();
    const captured = (await revisionOf(database, deployed.revisionId)).bindings.map((entry) => ({
        name: entry.name,
        target: entry.target,
        targetSpaceId: entry.targetSpaceId,
        version: entry.version,
    }));
    const boardRun = present(runtime.started.at(-1), "the run bound to the board");

    // report the missing address, run with the declared address, then with the other space's installation apart from the space's own resources
    expect({
        refused,
        declared: [observed.conditions["Ready"]?.status, observed.conditions["Ready"]?.reason],
        dependency: [dependency?.name, dependency?.reference, dependency?.provider],
        captured: captured.filter((entry) => entry.name === "tasks"),
        installations: boardRun.installations,
        dependencies: boardRun.resources.filter((entry) => entry.kind === "installation"),
    }).toEqual({
        refused: [
            "failed",
            `@destack/stack BINDING_BLOCKED: cannot apply installations '${ids.package} tasks': bind them to their addresses first`,
        ],
        declared: ["true", "Applied"],
        dependency: ["tasks", "notes", "http"],
        captured: [{ name: "tasks", target: board, targetSpaceId: acme, version: null }],
        installations: [
            {
                packageId: ids.package,
                name: "tasks",
                package: ids.package,
                installation: board,
                scope: acme,
            },
        ],
        dependencies: [],
    });
});

test("drop a draining release's columns once it retires", async () => {
    // run the first revision of notes whose notes have a color
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    let declared: readonly Declaration[] = [
        noteType({ body: field.string().optional(), color: field.string().optional() }),
    ];
    await installNotes(fixture, ids.notes, await notesBuild(declared, fixture.environment.crypto));
    await fixture.simulation.idle();

    // deploy a second revision that drops the color next to the first, and plan while the first drains
    declared = [main];
    await recordBuild(fixture, await notesBuild(declared, fixture.environment.crypto));
    await fixture.simulation.idle();
    const [stored] = await database.select().from(object.database.table);
    const [first] = await database
        .select({ retiredAt: object.deployment.table.retiredAt })
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);
    const ready = present(stored?.conditions["Ready"], "main's Ready");

    // plan dropping the color once the first release retires
    expect({
        isPlannedAfterRetirement:
            ready.lastTransitionAt >= present(first?.retiredAt, "the first retirement"),
        retired: [
            ready.reason,
            stored?.plan?.steps.map((step) => [step.action, step.risk, step.detail]) ?? null,
        ],
    }).toEqual({
        isPlannedAfterRetirement: true,
        retired: ["AwaitingApproval", [["replace", "destructive", "rebuild table: drop color"]]],
    });
});

test("convert narrowed values after stopping the draining release", async () => {
    // run the first revision of notes whose modes include emacs
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const { approve } = await serveControllers(fixture);
    const first = noteType({ mode: field.enum(["standard", "vim", "emacs"]) });
    const narrowed = noteType(
        { mode: field.enum(["standard", "vim"]) },
        {
            version: "2026.10.0",
            convert: {
                2: {
                    mode: Expression.case(
                        Expression.column("mode"),
                        [{ when: "emacs", then: Expression.literal("standard") }],
                        Expression.column("mode"),
                    ),
                },
            },
        },
    );
    let declared: readonly Declaration[] = [first];
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild(declared, fixture.environment.crypto),
    );
    await fixture.simulation.idle();
    const original = nth(await database.select().from(object.deployment.table), 0);
    await fixture.simulation.idle();

    // follow the narrowing revision a millisecond later, and plan a recreation that waits for approval
    await fixture.simulation.sleep(1);
    declared = [narrowed];
    await recordBuild(fixture, await notesBuild(declared, fixture.environment.crypto));
    await fixture.simulation.idle();
    const read = async () => {
        const row = nth(await database.select().from(object.database.table), 0);
        return {
            reason: row.conditions["Ready"]?.reason,
            plan: row.plan ?? undefined,
            id: row.id,
        };
    };
    const waiting = await read();

    // approve the recreation
    const approved = await approve(notes.installationId);
    await fixture.simulation.idle();
    const converted = await read();
    const [retired] = await database
        .select({ retiredAt: object.deployment.table.retiredAt })
        .from(object.deployment.table)
        .where(eq(object.deployment.table.id, original.id));
    const [stored] = await database.select().from(object.database.table);
    const appliedAt = present(stored?.conditions["Ready"], "main's Ready").lastTransitionAt;

    // deploy the replacement once main applied its converted tables
    await fixture.simulation.idle();
    const statuses = await database
        .select({ id: object.deployment.table.id, status: object.deployment.table.status })
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);

    // recreate only once approved with the first release stopped before converting
    expect({
        waiting: [waiting.reason, waiting.plan?.steps.map((step) => [step.action, step.risk])],
        approved,
        isStoppedBeforeConverted: present(retired?.retiredAt, "the first retirement") <= appliedAt,
        converted: converted.reason,
        statuses: statuses.map((entry) => entry.status),
    }).toEqual({
        waiting: [
            "AwaitingApproval",
            [
                ["delete", "backward-incompatible"],
                ["replace", "fallible"],
                ["convert", "fallible"],
            ],
        ],
        approved: {
            steps: present(waiting.plan, "the waiting plan").steps.map((step) => ({
                ...step,
                target: `database/main/${step.target}`,
            })),
        },
        isStoppedBeforeConverted: true,
        converted: "Applied",
        statuses: ["retired", "active"],
    });
});

test("grant an application its requested permissions until uninstalled", async () => {
    // install notes requesting reading the space and reading where people receive notifications
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const recipient = accountObject.user.permission("recipient");
    const request = requesting({
        space: { space: [space.permission("read")], universe: [recipient] },
    });
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, request], fixture.environment.crypto),
    );

    // apply it on a bun machine
    await fixture.simulation.idle();

    // read the installation's role in its space with its permissions, and the space's grants
    const installed = principal.installation.reference(ids.space, notes.installationId);
    const roles = async () =>
        (
            await database
                .select({
                    scope: accessRole.scope,
                    name: accessRole.name,
                    type: accessRolePermission.type,
                    permission: accessRolePermission.name,
                })
                .from(accessRole)
                .innerJoin(accessRolePermission, eq(accessRolePermission.roleId, accessRole.id))
                .where(eq(accessRole.name, `installation/${notes.installationId}`))
        ).map((row) => Object.values(row));
    const grants = async () =>
        (await database.select().from(accessGrant)).map((row) => ({
            delegator: ObjectReference.at("delegator", row),
            delegate: ObjectReference.at("delegate", row),
            permissions: row.permissions,
        }));
    const granted = { roles: await roles(), grants: await grants() };

    // uninstall it
    await changeInstallation(database, notes.installationId, {
        deletedAt: fixture.machine.clock.now(),
    });
    await fixture.simulation.idle();
    expect({
        granted,
        dropped: { roles: await roles(), grants: await grants() },
    }).toEqual({
        granted: {
            roles: [[ids.space, `installation/${notes.installationId}`, "space", "read"]],
            // lend the universe-scoped request on the space's authority, in every scope
            grants: [
                {
                    delegator: principal.space.reference(Scope.universe.id, ids.space),
                    delegate: installed,
                    permissions: [recipient],
                },
            ],
        },
        dropped: { roles: [], grants: [] },
    });
});

/** Run notes, hold a revision asking to read the space, and read the change request it opens. */
async function holdForAccess() {
    // run notes, then hold the next release's revision asking to read the space
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const { crypto } = fixture.environment;
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, requesting({})], crypto),
    );
    const { store } = await serveControllers(fixture);
    await fixture.simulation.idle();
    const request = requesting({ space: { space: [space.permission("read")] } });
    await recordBuild(
        fixture,
        await notesBuild([main, request], crypto, {}, ["main"], {}, "2026.10.0"),
    );
    await fixture.simulation.idle();

    // read the change request its rollout opened and the one approval it asks
    const [rolled] = await database
        .select()
        .from(object.rollout.table)
        .where(eq(object.rollout.table.installationId, notes.installationId));
    const opened = single(
        await database
            .select()
            .from(changeRequest.table)
            .where(eq(changeRequest.table.parentId, present(rolled, "the rollout").id)),
    );
    const asked = single(
        await database.select().from(approval.table).where(eq(approval.table.parentId, opened.id)),
    );

    // decide it as the space's owner, a manager, and let the change request act on the decision
    const decide = async (decision: "approve" | "reject") => {
        const owner = fixture.owner.subject;
        await store.mutate(AccessContext.of(Caller.of(owner), store.clock.now()), {
            id: RequestId.create(store.environment),
            calls: [approval.call(decision, approval.target(asked))],
        });
        await fixture.simulation.idle();
    };

    return { fixture, database, notes, opened, asked, decide };
}

test("grant held access once a manager approves its change request", async () => {
    // hold a revision asking to read the space, and read the plan it waits on
    const { fixture, database, notes, opened, asked, decide } = await holdForAccess();
    const held = await readInstalled(database, notes.installationId);
    const plan = present(held.installation.plan, "the waiting plan");

    // read the installation's role permissions before the approval
    const roles = async () =>
        (
            await database
                .select({ permission: accessRolePermission.name })
                .from(accessRole)
                .innerJoin(accessRolePermission, eq(accessRolePermission.roleId, accessRole.id))
                .where(eq(accessRole.name, `installation/${notes.installationId}`))
        ).map((row) => row.permission);
    const before = await roles();

    // approve it as a manager, then deploy the approved plan
    await decide("approve");
    await fixture.simulation.idle();
    const [decided] = await database
        .select()
        .from(changeRequest.table)
        .where(eq(changeRequest.table.id, opened.id));
    const applied = await readInstalled(database, notes.installationId);

    // wait on the grant as a privileged step, ask the managers as one set to approve exactly it, and grant it once approved
    expect({
        held: [held.installation.conditions["Ready"]?.reason, plan],
        before,
        asked: { ...ObjectReference.at("approver", asked), relation: asked.approverRelation },
        call: [opened.call.method, opened.call.input["plan"], ChangeRequest.as(opened)],
        state: decided?.state,
        applied: [applied.appliedRevisionId === applied.targetRevisionId, await roles()],
    }).toEqual({
        held: [
            "AwaitingApproval",
            {
                steps: [
                    {
                        action: "grant",
                        target: `installation/notes/release/2026.10.0/access/space/${space.permission("read").packageId}/space/read`,
                        risk: "privileged",
                        detail: "request read on space",
                    },
                ],
            },
        ],
        before: [],
        asked: { ...principal.space.reference(Scope.universe.id, ids.space), relation: "manage" },
        call: [
            "approve",
            await Plan.digest(plan, fixture.environment.crypto),
            { ...principal.space.reference(Scope.universe.id, ids.space), relation: "manage" },
        ],
        state: "applied",
        applied: [true, ["read"]],
    });
});

test("fail a held plan once a manager rejects it", async () => {
    // hold a revision asking to read the space, then reject its plan as a manager
    const { fixture, database, notes, opened, decide } = await holdForAccess();
    await decide("reject");
    await fixture.simulation.idle();
    const refused = single(fixture.takeRefusals());
    const held = await readInstalled(database, notes.installationId);
    const digest = Digest.parse(opened.call.input["plan"]);

    // fail the installation with the rejection, still running the revision applied before
    expect({
        refused,
        progress: [Observation.progress(held.installation), Observation.message(held.installation)],
        upgraded: held.appliedRevisionId === held.targetRevisionId,
    }).toEqual({
        refused: [
            "PRECONDITION_FAILED",
            "PLAN_BLOCKED",
            `cannot approve plan '${digest}': it was rejected, so change the installation for another plan`,
        ],
        progress: [
            "failed",
            `@destack/stack PLAN_BLOCKED: cannot approve plan '${digest}': it was rejected, so change the installation for another plan`,
        ],
        upgraded: false,
    });
});

test("lend an optional capability's permissions once allowed", async () => {
    // install notes declaring an optional push capability needing to unsubscribe people's browsers
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const unsubscribe = accountObject.client.permission("unsubscribe");
    const declared = requesting({});
    const request: Declaration = {
        ...declared,
        description: {
            ...declared.description,
            capabilities: {
                push: {
                    reason: t`Push notifications`,
                    optional: true,
                    permissions: { space: { universe: [unsubscribe] } },
                },
            },
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, request], fixture.environment.crypto),
    );
    const installed = principal.installation.reference(ids.space, notes.installationId);
    const grants = async () =>
        (await database.select().from(accessGrant)).map((row) => row.permissions);

    // deploy it, then allow push as its space would once it approves the request
    await fixture.simulation.idle();
    const asking = await grants();
    await changeInstallation(database, notes.installationId, { allowed: ["push"] });
    await fixture.simulation.idle();
    const enabled = (
        await database
            .select({
                enabled: object.deployment.table.enabled,
                status: object.deployment.table.status,
            })
            .from(object.deployment.table)
    ).toSorted((left, right) => left.status.localeCompare(right.status));

    // ask for push in approval mode on its own row, then run with it
    const allow = {
        ...installation.permission("allow"),
        scope: ids.space,
        objectId: notes.installationId,
        mode: "approval",
    };
    expect({ installed: installed.id, asking, allowed: await grants(), enabled }).toEqual({
        installed: notes.installationId,
        asking: [[allow]],
        // lend push's universe-scoped permission once allowed
        allowed: [[unsubscribe, allow]],
        enabled: [
            { enabled: ["push"], status: "active" },
            { enabled: [], status: "retired" },
        ],
    });
});

test("deploy a capability only once the account is entitled", async () => {
    // install notes requiring a sync capability that needs the plan's sync feature
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture, {
        isEnforcingEntitlements: true,
    });
    const feature = {
        packageId: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f1"),
        name: "sync",
    };
    const declared = requesting({});
    const request: Declaration = {
        ...declared,
        description: {
            ...declared.description,
            capabilities: { sync: { reason: t`Sync across devices`, features: [feature] } },
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, request], fixture.environment.crypto),
    );
    const deployed = async () =>
        (
            await database
                .select({ enabled: object.deployment.table.enabled })
                .from(object.deployment.table)
        ).map((row) => row.enabled);

    // deploy with the account replicated without the feature, then once it is entitled to it
    await TestEntitlement.set(
        database,
        ids.account,
        "database",
        { kind: "boolean" },
        fixture.machine.environment,
    );
    await fixture.simulation.idle();
    fixture.expectRefusals([
        [
            "PRECONDITION_FAILED",
            "CAPABILITY_BLOCKED",
            "cannot use capability 'sync': it needs 'sync': no plan of the account grants feature sync",
        ],
    ]);
    const refused = {
        ready: await readiness(database, notes.installationId),
        deployed: await deployed(),
    };
    await TestEntitlement.set(
        database,
        ids.account,
        { feature },
        { kind: "boolean" },
        fixture.machine.environment,
    );
    await fixture.simulation.idle();

    // refuse naming the missing feature, then run with the capability
    expect({ refused, entitled: await deployed() }).toEqual({
        refused: {
            ready: [
                "failed",
                "@destack/stack CAPABILITY_BLOCKED: cannot use capability 'sync': it needs 'sync': no plan of the account grants feature sync",
            ],
            deployed: [],
        },
        entitled: [["sync"]],
    });
});

test("deploy an application whose dependency its space installs", async () => {
    // install notes depending on the setting package, one of the space's own packages
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture, {});
    const notes = await installNotes(fixture);
    const { crypto } = fixture.environment;
    const depend = async (dependency: PackageVersion) =>
        await recordBuild(fixture, await notesBuild([main], crypto), [
            await dependencyBuild(dependency, crypto),
        ]);

    // depend on the setting package, then on the uninstalled vault
    await depend(PackageVersion.of(settingPackage));
    await fixture.simulation.idle();
    const deployed = await readiness(database, notes.installationId);
    await depend(VAULT);
    await fixture.simulation.idle();
    fixture.expectRefusals([
        ["PRECONDITION_FAILED", "DEPENDENCY_BLOCKED", "notes needs @acme/vault installed"],
    ]);
    const refused = await readiness(database, notes.installationId);

    // satisfy the space's own dependency, and refuse the one installed nowhere
    expect({ deployed, refused }).toEqual({
        deployed: ["current", "1 workloads deployed"],
        refused: ["failed", "@destack/stack DEPENDENCY_BLOCKED: notes needs @acme/vault installed"],
    });
});

test("refuse uninstalling a package an application needs", async () => {
    // install notes depending on a vault package its space lacks, published at the registry
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const { uninstall } = await serveControllers(fixture, {});
    await fixture.registry.release(await MemoryBuild.of(VAULT, [], testHost().crypto));
    const { crypto } = fixture.environment;
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main], crypto), [
        await dependencyBuild(VAULT, crypto),
    ]);

    // deploy without the vault, then once installing it wakes notes through the controllers' watches
    await fixture.simulation.idle();
    fixture.expectRefusals([
        ["PRECONDITION_FAILED", "DEPENDENCY_BLOCKED", "notes needs @acme/vault installed"],
    ]);
    const refused = await readiness(database, notes.installationId);
    const now = fixture.machine.clock.now();
    const installed = await insertRollout(
        database,
        {
            id: Identifier.create("installation", fixture.environment),
            scope: ids.space,
            packageId: VAULT.id,
            name: "vault",
            follows: TestRegistry.row("release", VAULT.version),
            installer: fixture.owner.subject,
        },
        now,
    );
    await fixture.simulation.idle();
    const deployed = await readiness(database, notes.installationId);

    // uninstall the vault while notes needs it
    const uninstalled = await refusal(uninstall(installed.installationId));

    // refuse with the missing package's name, deploy once installed, and leave the vault installed
    expect({ refused, deployed, uninstalled }).toEqual({
        refused: ["failed", "@destack/stack DEPENDENCY_BLOCKED: notes needs @acme/vault installed"],
        deployed: ["current", "1 workloads deployed"],
        uninstalled: [
            "PRECONDITION_FAILED",
            "DEPENDENT_BLOCKED",
            "cannot uninstall 'vault': 'notes' needs it",
        ],
    });
});

test("refuse an allowed capability until its package is installed", async () => {
    // install notes with an optional vault sync capability needing a vault package its space lacks
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const declared = requesting({});
    const request: Declaration = {
        ...declared,
        description: {
            ...declared.description,
            capabilities: {
                "vault-sync": {
                    reason: t`Keep notes in the vault`,
                    optional: true,
                    packages: [VAULT.id],
                },
            },
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, request], fixture.environment.crypto),
    );
    const vault = await fixture.registry.release(
        await MemoryBuild.of(VAULT, [], testHost().crypto),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.installationRevision.table)
                .set({ dependencies: [{ package: VAULT, manifest: Digest.parse(vault) }] })
                .where(eq(object.installationRevision.table.installationId, notes.installationId)),
    );

    // deploy without the capability, then with it allowed
    await fixture.simulation.idle();
    const without = await readiness(database, notes.installationId);
    await changeInstallation(database, notes.installationId, { allowed: ["vault-sync"] });
    await fixture.simulation.idle();
    fixture.expectRefusals([
        [
            "PRECONDITION_FAILED",
            "DEPENDENCY_BLOCKED",
            "capability vault-sync needs @acme/vault installed",
        ],
    ]);
    const allowed = await readiness(database, notes.installationId);

    // deploy without the vault until the capability needs it
    expect({ without, allowed }).toEqual({
        without: ["current", "1 workloads deployed"],
        allowed: [
            "failed",
            "@destack/stack DEPENDENCY_BLOCKED: capability vault-sync needs @acme/vault installed",
        ],
    });
});

test("wait for a provider of an unserved resource kind", async () => {
    // install notes binding a vault, whose provider no installation here serves yet
    const fixture = await TestSpace.open();
    await serveControllers(fixture);
    const credentials: Declaration = {
        resource: VaultKind,
        name: "credentials",
        description: { name: "credentials", spec: {} },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, credentials], fixture.environment.crypto),
    );

    // wait rather than fail, naming the kind
    await fixture.simulation.idle();
    expect(await readiness(fixture.database, notes.installationId)).toEqual([
        "waiting",
        "waiting for a provider of vault resources",
    ]);
});

test("wait for a provider of a selected object's kind", async () => {
    // install notes needing a connection, and select an existing one before any server here serves connections
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await serveControllers(fixture);
    const github: Declaration = {
        resource: ConnectionKind,
        name: "github",
        description: { name: "github", spec: {} },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, github], fixture.environment.crypto),
    );
    await selectBinding(database, notes.installationId, "github", {
        type: "connection",
        id: Identifier.create("connection", fixture.environment),
    });

    // wait rather than fail the revision, naming the kind
    await fixture.simulation.idle();
    expect(await readiness(database, notes.installationId)).toEqual([
        "waiting",
        "waiting for a provider of connection resources",
    ]);
});

/** Read an installation's Ready condition, apart from when it last changed. */
function readyOf(installed: Awaited<ReturnType<typeof readInstalled>>) {
    const { lastTransitionAt: _changed, ...condition } = present(
        installed.installation.conditions["Ready"],
        "a Ready",
    );

    return condition;
}
